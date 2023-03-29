use std::{net::SocketAddr, sync::Arc};

use futures_util::{pin_mut, StreamExt, TryStreamExt};
use hyper::upgrade::Upgraded;
use nostr_core::Request;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedSender},
    Mutex,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::{Connections, Context};

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
            let res = UnboundedReceiverStream::new(rx)
                .map(Ok)
                .forward(outgoing)
                .await;
            if let Err(e) = res {
                tracing::error!(?e, "Error forwarding messages");
            }

            let connection_ref = connections_ref.get_connection_mut(addr).await;
            if let Some(mut c) = connection_ref {
                c.status = Status::Closed;
                c.remove();
            };
        });

        let connections_ref = ctx.connections.clone();
        tokio::spawn(async move {
            while let Some(msg) = incoming.next().await {
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
                                c.tx.send(Message::Close(None)).ok();
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
        });
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn close(&mut self) {
        self.tx.send(Message::Close(None)).ok();
        self.status = Status::CloseRequesting;
    }
}
