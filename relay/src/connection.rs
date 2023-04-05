use std::{collections::HashMap, net::SocketAddr};

use futures_util::{pin_mut, Sink, Stream, StreamExt};
use hyper::upgrade::Upgraded;
use nostr_core::{Filters, Request, SubscriptionId};
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
    subscriptions: HashMap<SubscriptionId, Filters>,
}

impl Connection {
    pub async fn new(ctx: Context, ws_stream: WebSocketStream<Upgraded>, addr: SocketAddr) {
        let (tx, rx) = unbounded_channel();
        let (outgoing, incoming) = ws_stream.split();
        let connection = Self {
            addr,
            tx,
            status: Status::Connected,
            subscriptions: HashMap::new(),
        };
        ctx.connections.insert(connection).await;

        Self::spawn_outgoing_stream(rx, outgoing);
        tokio::spawn(Self::handle_incoming_stream(ctx, addr, incoming));
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

    #[tracing::instrument(name = "incoming", skip(ctx, incoming))]
    async fn handle_incoming_stream<S>(ctx: Context, addr: SocketAddr, incoming: S)
    where
        S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Send + 'static,
    {
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
                    if let Err(error) = Self::handle_request(ctx.clone(), addr, req).await {
                        tracing::error!(?error, "handle request error");
                    }
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
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn close(&mut self) {
        self.send_raw(Message::Close(None));
        self.status = Status::CloseRequesting;
    }

    #[tracing::instrument(skip_all, fields(r#type = req.type_str()))]
    async fn handle_request(ctx: Context, addr: SocketAddr, req: Request) -> anyhow::Result<()> {
        tracing::info!("{req:?}");

        match req {
            Request::Event(event) => {
                if !event.verify() {
                    tracing::info!("verify failed");
                    return Ok(());
                }
                let item = serde_dynamo::to_item(event)?;
                ctx.dynamodb
                    .put_item()
                    .table_name("events")
                    .set_item(Some(item))
                    .send()
                    .await?;
            }
            Request::Req {
                subscription_id,
                filters,
            } => {
                let Some(mut connection) = ctx.connections.get_connection_mut(addr).await else {
                    tracing::warn!("connection not found");
                    return Ok(());
                };

                connection.subscriptions.insert(subscription_id, filters);
                tracing::info!("{:?}", connection.subscriptions);
            }
            Request::Close(subscription_id) => {
                let Some(mut connection) = ctx.connections.get_connection_mut(addr).await else {
                    tracing::warn!("connection not found");
                    return Ok(());
                };

                connection.subscriptions.remove(&subscription_id);
                tracing::info!("{:?}", connection.subscriptions);
            }
        }

        Ok(())
    }

    fn send_raw(&self, message: Message) {
        tracing::info!(?message, "send");
        self.tx.send(message).ok();
    }
}
