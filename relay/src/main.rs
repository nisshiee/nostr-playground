use std::{convert::Infallible, net::SocketAddr, time::Duration};

use copy_from_relay::{copy_from_relay, Stop};
use futures_util::{FutureExt, StreamExt};
use hyper::{
    header::{
        ACCEPT, ACCESS_CONTROL_ALLOW_ORIGIN, CONNECTION, CONTENT_TYPE, SEC_WEBSOCKET_ACCEPT,
        SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
    },
    http::HeaderValue,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    upgrade::Upgraded,
    Method, Server, StatusCode, Version,
};
use nostr_core::{Pubkey, RelayInformation};
use signal_hook::consts::SIGINT;
use signal_hook_tokio::Signals;
use tokio_tungstenite::{
    tungstenite::{handshake::derive_accept_key, protocol::Role},
    WebSocketStream,
};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

mod connection;
pub use connection::Connection;

mod connections;
pub use connections::Connections;

mod context;
pub use context::Context;

mod query;
pub use query::Query;

mod copy_from_relay;

#[cfg(debug_assertions)]
const BIND_HOST: &str = "127.0.0.1:8080";
#[cfg(not(debug_assertions))]
const BIND_HOST: &str = "0.0.0.0:80";

const MY_PUBKEY: Pubkey = Pubkey::new([
    0x73, 0x49, 0x15, 0x09, 0xb8, 0xe2, 0xd8, 0x08, 0x40, 0x87, 0x3b, 0x5a, 0x13, 0xba, 0x98, 0xa5,
    0xd1, 0xac, 0x3a, 0x16, 0xc9, 0x29, 0x2e, 0x10, 0x6b, 0x1f, 0x2e, 0xda, 0x31, 0x15, 0x2c, 0x52,
]);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_ansi(false)
        .without_time()
        .init();

    let addr = BIND_HOST.parse().unwrap();

    let ctx = Context::new().await;

    let signals = Signals::new([SIGINT])?;
    let signals_handle = signals.handle();
    let signal_task = tokio::spawn(handle_signals(signals)).map(|r| {
        if let Err(error) = r {
            tracing::error!(?error);
        }
    });

    let copy_from_relay = copy_from_relay(ctx.clone());

    let ctx_for_service = ctx.clone();
    let make_svc = make_service_fn(move |conn: &AddrStream| {
        let remote_addr = conn.remote_addr();
        let ctx_for_service = ctx_for_service.clone();
        let service =
            service_fn(move |req| handle_request(ctx_for_service.clone(), req, remote_addr));
        async { Ok::<_, Infallible>(service) }
    });

    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(signal_task);

    server.await?;
    ctx.connections.close_all().await;
    signals_handle.close();
    copy_from_relay.send(Stop).ok();
    tokio::time::sleep(Duration::from_secs(5)).await;
    Ok(())
}

#[tracing::instrument(skip(ctx, req), fields(method = %req.method(), path = %req.uri().path()))]
async fn handle_request(
    ctx: Context,
    mut req: hyper::Request<hyper::Body>,
    addr: SocketAddr,
) -> Result<hyper::Response<hyper::Body>, Infallible> {
    tracing::info!("{req:?}");
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let headers = req.headers();

            if headers
                .get(ACCEPT)
                .and_then(|h| h.to_str().ok())
                .map(|v| v.to_lowercase().contains("application/nostr+json"))
                .unwrap_or(false)
            {
                tracing::info!("returning relay information document: NIP-11");
                let mut info = RelayInformation::default();
                info.name = Some("Dev Relay".to_owned());
                info.description = Some("WARNING! This relay is under development.".to_owned());
                info.pubkey = Some(MY_PUBKEY);
                info.supported_nips = Some(vec![1, 11, 15]);

                let mut res = hyper::Response::new(hyper::Body::empty());
                *res.body_mut() = hyper::Body::from(serde_json::to_string(&info).unwrap());
                res.headers_mut()
                    .append(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
                res.headers_mut().append(
                    CONTENT_TYPE,
                    HeaderValue::from_static("application/nostr+json; charset=utf-8"),
                );
                return Ok(res);
            }

            let upgrade = HeaderValue::from_static("Upgrade");
            let websocket = HeaderValue::from_static("websocket");
            let key = headers.get(SEC_WEBSOCKET_KEY);
            let derived = key.map(|k| derive_accept_key(k.as_bytes()));
            if req.version() < Version::HTTP_11
                || !headers
                    .get(CONNECTION)
                    .and_then(|h| h.to_str().ok())
                    .map(|h| {
                        h.split(|c| c == ' ' || c == ',')
                            .any(|p| p.eq_ignore_ascii_case(upgrade.to_str().unwrap()))
                    })
                    .unwrap_or(false)
                || !headers
                    .get(UPGRADE)
                    .and_then(|h| h.to_str().ok())
                    .map(|h| h.eq_ignore_ascii_case("websocket"))
                    .unwrap_or(false)
                || !headers
                    .get(SEC_WEBSOCKET_VERSION)
                    .map(|h| h == "13")
                    .unwrap_or(false)
                || key.is_none()
            {
                tracing::info!("returning web page");
                return Ok(hyper::Response::new(hyper::Body::from("Hello World!")));
            }

            tracing::info!("upgrade to websocket connection");
            let ver = req.version();
            tokio::task::spawn(async move {
                match hyper::upgrade::on(&mut req).await {
                    Ok(upgraded) => {
                        let _ = handle_connection(
                            ctx.clone(),
                            WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await,
                            addr,
                        )
                        .await;
                    }
                    Err(e) => tracing::info!(?e, "upgrade error"),
                }
            });
            let mut res = hyper::Response::new(hyper::Body::empty());
            *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
            *res.version_mut() = ver;
            res.headers_mut().append(CONNECTION, upgrade);
            res.headers_mut().append(UPGRADE, websocket);
            res.headers_mut()
                .append(SEC_WEBSOCKET_ACCEPT, derived.unwrap().parse().unwrap());
            Ok(res)
        }
        (&Method::GET, "/health") => {
            let mut res = hyper::Response::new(hyper::Body::from("OK"));
            *res.status_mut() = StatusCode::OK;
            res.headers_mut()
                .append(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
            Ok(res)
        }
        _ => {
            let mut res = hyper::Response::new(hyper::Body::empty());
            *res.status_mut() = StatusCode::NOT_FOUND;
            Ok(res)
        }
    }
}

#[tracing::instrument(skip(ctx, ws_stream))]
async fn handle_connection(
    ctx: Context,
    ws_stream: WebSocketStream<Upgraded>,
    addr: SocketAddr,
) -> anyhow::Result<()> {
    Connection::new(ctx, ws_stream, addr).await;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_signals(mut signals: Signals) {
    while let Some(signal) = signals.next().await {
        tracing::info!("received signal: {}", signal);
        match signal {
            SIGINT => {
                tracing::info!("shutting down...");
                break;
            }
            _ => unreachable!(),
        }
    }
}
