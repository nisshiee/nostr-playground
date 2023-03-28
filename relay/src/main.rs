use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::Arc};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{FutureExt, SinkExt, StreamExt, TryStreamExt};
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
use nostr_core::{RelayInformation, Request};
use signal_hook::consts::SIGINT;
use signal_hook_tokio::Signals;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    tungstenite::{handshake::derive_accept_key, protocol::Role, Message},
    WebSocketStream,
};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

#[cfg(debug_assertions)]
const BIND_HOST: &str = "127.0.0.1:8080";
#[cfg(not(debug_assertions))]
const BIND_HOST: &str = "0.0.0.0:80";

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_ansi(false)
        .without_time()
        .init();

    let addr = BIND_HOST.parse().unwrap();

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    let signals = Signals::new([SIGINT])?;
    let signals_handle = signals.handle();
    let signal_task = tokio::spawn(handle_signals(state.clone(), signals)).map(|r| {
        if let Err(e) = r {
            tracing::error!("{e:?}");
        }
    });

    let make_svc = make_service_fn(move |conn: &AddrStream| {
        let remote_addr = conn.remote_addr();
        let state = state.clone();
        let service = service_fn(move |req| handle_request(state.clone(), req, remote_addr));
        async { Ok::<_, Infallible>(service) }
    });

    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(signal_task);

    server.await?;
    signals_handle.close();
    Ok(())
}

#[tracing::instrument(skip(peer_map))]
async fn handle_request(
    peer_map: PeerMap,
    mut req: hyper::Request<hyper::Body>,
    addr: SocketAddr,
) -> Result<hyper::Response<hyper::Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let headers = req.headers();

            if headers
                .get(ACCEPT)
                .and_then(|h| h.to_str().ok())
                .map(|v| v.to_lowercase().contains("application/nostr+json"))
                .unwrap_or(false)
            {
                let mut info = RelayInformation::default();
                info.name = Some("Dev Relay".to_owned());
                info.description = Some("WARNING! This relay is under development.".to_owned());

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
                return Ok(hyper::Response::new(hyper::Body::from("Hello World!")));
            }
            let ver = req.version();
            tokio::task::spawn(async move {
                match hyper::upgrade::on(&mut req).await {
                    Ok(upgraded) => {
                        let _ = handle_connection(
                            peer_map,
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

#[tracing::instrument(skip(peer_map, ws_stream))]
async fn handle_connection(
    peer_map: PeerMap,
    ws_stream: WebSocketStream<Upgraded>,
    addr: SocketAddr,
) -> anyhow::Result<()> {
    tracing::info!("WebSocket connection established: {}", addr);

    let (tx, _rx) = unbounded();
    peer_map.lock().await.insert(addr, tx);

    let (mut outgoing, incoming) = ws_stream.split();

    let incoming = incoming.try_for_each(|msg| async {
        if let Err(err) = handle_message(msg).await {
            tracing::error!(error = ?err, "Error handling message");
        }
        Ok(())
    });

    outgoing.send(Message::Ping(vec![])).await.ok();

    incoming.await?;

    tracing::info!("disconnected");
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_message(msg: Message) -> anyhow::Result<()> {
    tracing::debug!(message = ?msg, "raw message");
    match msg {
        Message::Text(text) => {
            let req = serde_json::from_str::<Request>(&text)?;
            tracing::info!(request = ?req, "request");
        }
        _ => {
            tracing::debug!("ignore non-text message");
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_signals(peer_map: PeerMap, mut signals: Signals) {
    while let Some(signal) = signals.next().await {
        tracing::info!("received signal: {}", signal);
        match signal {
            SIGINT => {
                tracing::info!("shutting down...");
                let mut peers = peer_map.lock().await;
                for peer in peers.values() {
                    let _ = peer.unbounded_send(Message::Close(None));
                    peer.close_channel();
                }
                peers.clear();
                break;
            }
            _ => unreachable!(),
        }
    }
}
