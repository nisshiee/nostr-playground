use std::net::SocketAddr;

use futures_util::StreamExt;
use hyper::upgrade::Upgraded;
use nostr_core::Request;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::Context;

pub type Tx = UnboundedSender<Message>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Connected,
    CloseRequesting,
    Closed,
}

#[derive(Clone)]
pub struct Connection {
    addr: SocketAddr,
    tx: Tx,
    status: Status,
}

impl Connection {
    pub async fn new(
        ctx: Context,
        ws_stream: WebSocketStream<Upgraded>,
        addr: SocketAddr,
        mut handle_request: impl FnMut(Context, Request) -> anyhow::Result<()> + Send + 'static,
    ) {
        tracing::info!("WebSocket connection established: {}", addr);

        let (tx, rx) = unbounded_channel();
        let (outgoing, mut incoming) = ws_stream.split();
        let connection = Self {
            addr,
            tx,
            status: Status::Connected,
        };
        ctx.connections.insert(connection).await;

        let connections_ref = ctx.connections.clone();
        tokio::spawn(async move {
            UnboundedReceiverStream::new(rx)
                .map(Ok)
                .forward(outgoing)
                .await
                .ok();

            let connection_ref = connections_ref.get_connection_mut(addr).await;
            if let Some(mut c) = connection_ref {
                c.status = Status::Closed;
                c.remove();
            };
        });

        let connections_ref = ctx.connections.clone();
        tokio::spawn(async move {
            while let Some(msg) = incoming.next().await {
                if let Ok(ref msg) = msg {
                    tracing::info!(message = ?msg, "received");
                }
                match msg {
                    Ok(Message::Text(text)) => {
                        let req = match serde_json::from_str::<Request>(&text) {
                            Ok(req) => req,
                            Err(error) => {
                                tracing::error!(?text, ?error, "Error parsing request");
                                continue;
                            }
                        };
                        tracing::info!(request = ?req, "request");
                        if let Err(error) = handle_request(ctx.clone(), req) {
                            tracing::error!(?error, "Error handling request");
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("closing connection");
                        let connection_ref = connections_ref.get_connection_mut(addr).await;
                        if let Some(mut c) = connection_ref {
                            if c.status == Status::CloseRequesting {
                                tracing::info!("receive reply close handshake");
                            } else {
                                c.send_raw(Message::Close(None));
                            }
                            c.status = Status::Closed;
                            c.remove();
                        }
                        break;
                    }
                    // TODO: PING, PONG, CLOSEを良い感じに
                    Ok(_) => {
                        tracing::debug!("ignore non-text message");
                    }
                    Err(e) => {
                        tracing::error!(?e, "Error receiving message");
                        break;
                    }
                }
            }

            if let Some(mut c) = ctx.connections.get_connection_mut(addr).await {
                c.status = Status::Closed;
                c.remove();
            }
            tracing::info!("disconnected");
        });
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn close(&mut self) {
        self.send_raw(Message::Close(None));
        self.status = Status::CloseRequesting;
    }

    fn send_raw(&self, message: Message) {
        tracing::info!(?message, "send");
        self.tx.send(message).ok();
    }
}
