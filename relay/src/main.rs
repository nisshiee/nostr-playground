use std::net::SocketAddr;

use futures_util::{pin_mut, StreamExt, TryStreamExt};
use nostr_core::Request;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

// https://github.com/snapview/tokio-tungstenite/blob/master/examples/server.rs

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }

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
