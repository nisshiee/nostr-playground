use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{
    future::{self, Either},
    pin_mut, StreamExt, TryStreamExt,
};
use nostr_core::Request;
use signal_hook::consts::SIGINT;
use signal_hook_tokio::Signals;
use tokio::{
    net::{TcpListener, TcpStream},
    pin,
    sync::Mutex,
};
use tokio_tungstenite::tungstenite::Message;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

// https://github.com/snapview/tokio-tungstenite/blob/master/examples/server.rs

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
        .init();

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    let signals = Signals::new(&[SIGINT])?;
    let signals_handle = signals.handle();
    let signal_task = tokio::spawn(handle_signals(state.clone(), signals));
    pin_mut!(signal_task);

    let listener = TcpListener::bind(BIND_HOST).await?;

    loop {
        let connection = listener.accept();
        pin!(connection);
        match future::select(connection, signal_task).await {
            Either::Left((Ok((stream, addr)), t)) => {
                tokio::spawn(handle_connection(stream, addr));
                signal_task = t;
            }
            Either::Right((Ok(()), _)) => {
                tracing::info!("shutting down");
                break;
            }
            _ => unimplemented!(), // TODO: Errが返ったときの処理も書いて、panicさせないように
        }
    }
    // while let Ok((stream, addr)) = listener.accept().await {
    //     tokio::spawn(handle_connection(stream, addr));
    // }

    signals_handle.close();
    Ok(())
}

#[tracing::instrument(skip(raw_stream))]
async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) -> anyhow::Result<()> {
    tracing::debug!("Incoming TCP connection");

    let ws_stream = tokio_tungstenite::accept_async(raw_stream).await?;
    tracing::info!("WebSocket connection established");

    let (_outgoing, incoming) = ws_stream.split();

    let incoming = incoming.try_for_each(|msg| async {
        if let Err(err) = handle_message(msg).await {
            tracing::error!(error = ?err, "Error handling message");
        }
        Ok(())
    });

    pin_mut!(incoming);
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
