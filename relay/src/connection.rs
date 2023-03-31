use std::net::SocketAddr;

use futures_util::{pin_mut, Sink, Stream, StreamExt};
use hyper::upgrade::Upgraded;
use nostr_core::Request;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
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
    pub async fn new(ctx: Context, ws_stream: WebSocketStream<Upgraded>, addr: SocketAddr) {
        let (tx, rx) = unbounded_channel();
        let (outgoing, incoming) = ws_stream.split();
        let connection = Self {
            addr,
            tx,
            status: Status::Connected,
        };
        ctx.connections.insert(connection).await;

        Self::spawn_outgoing_stream(rx, outgoing);
        Self::spawn_incoming_stream(ctx, addr, incoming);
    }

    fn spawn_outgoing_stream<S>(rx: UnboundedReceiver<Message>, outgoing: S)
    where
        S: Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Send + 'static,
    {
        tokio::spawn(async move {
            UnboundedReceiverStream::new(rx)
                .map(Ok)
                .forward(outgoing)
                .await
                .ok();
        });
    }

    fn spawn_incoming_stream<S>(ctx: Context, addr: SocketAddr, incoming: S)
    where
        S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Send + 'static,
    {
        tokio::spawn(async move {
            let span = tracing::info_span!("incoming", ?addr);
            let _enter = span.enter();

            pin_mut!(incoming);
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
                        Self::handle_request(ctx.clone(), addr, req).await;
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("closing connection");
                        let connection = ctx.connections.get_connection_mut(addr).await;
                        if let Some(mut c) = connection {
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
                    Ok(Message::Ping(_)) => {
                        if let Some(c) = ctx.connections.get_connection_mut(addr).await {
                            c.send_raw(Message::Pong(vec![]));
                        }
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

    #[tracing::instrument(skip_all, fields(r#type = req.type_str()))]
    async fn handle_request(ctx: Context, addr: SocketAddr, req: Request) {
        tracing::info!("{req:?}");

        if let Request::Event(event) = req {
            ctx.dynamodb
                .put_item()
                .table_name("events")
                .item("id", event.id)
        }
    }

    fn send_raw(&self, message: Message) {
        tracing::info!(?message, "send");
        self.tx.send(message).ok();
    }
}
