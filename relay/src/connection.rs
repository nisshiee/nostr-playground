use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use futures_util::{pin_mut, Sink, Stream, StreamExt};
use hyper::upgrade::Upgraded;
use nostr_core::{Request, Response, SubscriptionId};
use tokio::sync::{
    broadcast::error::RecvError,
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use ulid::Ulid;

use crate::{Context, Query};

pub type Tx = UnboundedSender<Message>;

#[derive(Clone)]
pub enum Status {
    Connected {
        subscriptions: HashMap<SubscriptionId, Ulid>,
    },
    CloseRequesting,
    Closed,
}

impl Status {
    pub fn is_close_requesting(&self) -> bool {
        matches!(self, Self::CloseRequesting)
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::Connected {
            subscriptions: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct Connection {
    addr: SocketAddr,
    tx: Tx,
    status: Arc<Mutex<Status>>,
}

impl Connection {
    pub async fn new(ctx: Context, ws_stream: WebSocketStream<Upgraded>, addr: SocketAddr) {
        let (tx, rx) = unbounded_channel();
        let (outgoing, incoming) = ws_stream.split();
        let connection = Self {
            addr,
            tx,
            status: Arc::new(Mutex::new(Status::default())),
        };
        ctx.connections.insert(connection.clone()).await;

        Self::spawn_outgoing_stream(rx, outgoing);
        tokio::spawn(Self::handle_incoming_stream(ctx, connection, incoming));
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

    #[tracing::instrument(name = "incoming", skip_all, fields(addr = conn.addr().to_string()))]
    async fn handle_incoming_stream<S>(ctx: Context, conn: Connection, incoming: S)
    where
        S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Send + 'static,
    {
        let addr = conn.addr();

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
                    if let Err(error) = Self::handle_request(ctx.clone(), conn.clone(), req).await {
                        tracing::error!(?error, "handle request error");
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("closing connection");
                    let connection = ctx.connections.get_connection_mut(addr).await;
                    let mut status = conn.status.lock().await;

                    if status.is_close_requesting() {
                        tracing::info!("receive reply close handshake");
                    } else {
                        conn.send_raw(Message::Close(None));
                    }
                    *status = Status::Closed;

                    if let Some(c) = connection {
                        c.remove()
                    }
                    break;
                }
                Ok(_) => {
                    tracing::debug!("ignore non-text message");
                }
                Err(e) => {
                    tracing::error!(?e, "Error receiving message");
                    break;
                }
            }
        }

        let connection = ctx.connections.get_connection_mut(addr).await;
        let mut status = conn.status.lock().await;
        *status = Status::Closed;
        if let Some(c) = connection {
            c.remove()
        }

        tracing::info!("disconnected");
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn close(&mut self) {
        let mut status = self.status.lock().await;
        self.send_raw(Message::Close(None));
        *status = Status::CloseRequesting;
    }

    #[tracing::instrument(skip_all, fields(r#type = req.type_str()))]
    async fn handle_request(ctx: Context, conn: Connection, req: Request) -> anyhow::Result<()> {
        tracing::info!("{req:?}");

        match req {
            Request::Event(event) => {
                if !event.verify() {
                    tracing::info!("verify failed");
                    return Ok(());
                }
                if let Err(error) = ctx.event_broadcast.send(event.clone()) {
                    tracing::error!(?error, "event broadcast error");
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
                let query = Query::new(filters.min_since(), filters.max_until());
                let events = query.exec(&ctx).await?;
                let events = events.into_iter().filter(|e| filters.is_fit(e));
                for event in events {
                    let response = Response::Event {
                        subscription_id: subscription_id.clone(),
                        event,
                    };
                    let message = Message::Text(serde_json::to_string(&response)?);
                    conn.send_raw(message);
                }
                if let Status::Connected { subscriptions } = &mut *conn.status.lock().await {
                    let ulid = Ulid::new();
                    subscriptions.insert(subscription_id.clone(), ulid);
                    tracing::info!("{:?}", subscriptions);

                    let conn = conn.clone();
                    let mut rx = ctx.event_broadcast.subscribe();
                    tokio::spawn(async move {
                        loop {
                            tokio::select! {
                                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                                    let Status::Connected { subscriptions } = &*conn.status.lock().await else { break; };
                                    let Some(existing_ulid) = subscriptions.get(&subscription_id) else { break; };
                                    if existing_ulid != &ulid {
                                        break;
                                    }
                                }
                                event = rx.recv() => match event {
                                    Ok(event) => {
                                        if filters.is_fit(&event) {
                                            let response = Response::Event {
                                                subscription_id: subscription_id.clone(),
                                                event,
                                            };
                                            match serde_json::to_string(&response) {
                                                Ok(response) => {
                                                    let message = Message::Text(response);
                                                    conn.send_raw(message);
                                                }
                                                Err(error) => {
                                                    tracing::error!(?error, "Error serializing response");
                                                }
                                            }
                                        }
                                    }
                                    Err(RecvError::Closed) => {
                                        tracing::info!("event broadcast closed");
                                        break;
                                    }
                                    _ => {},
                                }
                            }
                        }
                    });
                }
            }
            Request::Close(subscription_id) => {
                if let Status::Connected { subscriptions } = &mut *conn.status.lock().await {
                    subscriptions.remove(&subscription_id);
                    tracing::info!("{:?}", subscriptions);
                }
            }
        }

        Ok(())
    }

    fn send_raw(&self, message: Message) {
        tracing::info!(?message, "send");
        self.tx.send(message).ok();
    }
}
