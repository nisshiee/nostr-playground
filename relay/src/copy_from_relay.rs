use std::{collections::HashMap, str::FromStr};

use aws_sdk_dynamodb::model::AttributeValue;
use chrono::Utc;
use futures_util::{Sink, SinkExt, Stream, StreamExt, TryFutureExt};
use nostr_core::{Filter, Filters, HexPrefix, Pubkey, RawEvent, Request, Response, SubscriptionId};
use serde_dynamo::to_attribute_value;
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::Error as WsError, tungstenite::Message};
use ulid::Ulid;

use crate::{Context, MY_PUBKEY};

pub struct Stop;

const RELAY_URL: &str = "wss://nostr.wine";

#[tracing::instrument(skip_all)]
pub fn copy_from_relay(ctx: Context) -> oneshot::Sender<Stop> {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(subscribe_relay(ctx, rx).map_err(|e| tracing::error!("{:?}", e)));
    tx
}

#[tracing::instrument(skip_all)]
async fn get_followings(ctx: &Context) -> anyhow::Result<Vec<Pubkey>> {
    let res = ctx
        .dynamodb
        .get_item()
        .table_name("contact_lists")
        .key("pubkey", to_attribute_value(MY_PUBKEY)?)
        .send()
        .await?;
    let item = res.item.ok_or(anyhow::anyhow!("not found"))?;
    let contact_list_event: RawEvent = serde_dynamo::from_item(item)?;
    let contact_list = contact_list_event
        .tags
        .into_iter()
        .filter_map(|tag| {
            if tag.name == "p" {
                match Pubkey::from_str(&tag.value) {
                    Ok(pubkey) => Some(pubkey),
                    Err(error) => {
                        tracing::error!(?error, "invalid pubkey");
                        None
                    }
                }
            } else {
                None
            }
        })
        .collect();
    Ok(contact_list)
}

#[tracing::instrument(skip_all)]
async fn subscribe_relay(ctx: Context, mut stop: oneshot::Receiver<Stop>) -> anyhow::Result<()> {
    let (mut write, mut read) = open_connection(&ctx).await?;

    loop {
        tokio::select! {
            Some(message) = read.next() => match message {
                Ok(message) => {
                    tracing::info!(?message, "message");
                    match message {
                        Message::Ping(_) => {
                            if let Err(_) = write.send(Message::Pong(vec![])).await {
                                (write, read) = open_connection(&ctx).await?;
                                continue;
                            }
                        }
                        Message::Close(_) => {
                            write.send(Message::Close(None)).await.ok();
                            (write, read) = open_connection(&ctx).await?;
                            continue;
                        }
                        Message::Text(text) => {
                            let event: Response = match serde_json::from_str(&text) {
                                Ok(event) => event,
                                Err(error) => {
                                    tracing::error!(?error, "deserialize response failed");
                                    continue;
                                }
                            };

                            let event = match event {
                                Response::Event { event, .. } => event,
                                _ => { continue; }
                            };
                            if !event.verify() { continue; }

                            ctx.event_broadcast.send(event.clone()).ok();

                            let kind = event.kind;
                            let created_at = event.created_at;

                            let item: HashMap<String, AttributeValue> = serde_dynamo::to_item(event)?;
                            if kind == 3 {
                                ctx.dynamodb
                                    .put_item()
                                    .table_name("contact_lists")
                                    .set_item(Some(item.clone()))
                                    .condition_expression(
                                        "created_at < :created_at OR attribute_not_exists(pubkey)",
                                    )
                                    .expression_attribute_values(
                                        ":created_at",
                                        to_attribute_value(created_at.timestamp()).unwrap(),
                                    )
                                    .send()
                                    .await?;
                            }
                            ctx.dynamodb
                                .put_item()
                                .table_name("events")
                                .set_item(Some(item))
                                .send()
                                .await?;
                        }
                        _ => {}
                    }
                }
                Err(error) => {
                    tracing::error!(?error, "error");
                    (write, read) = open_connection(&ctx).await?;
                    continue;
                }
            },
            _ = &mut stop => {
                tracing::info!("stop");
                write.send(Message::Close(None)).await?;
                break;
            }
            else => {
                (write, read) = open_connection(&ctx).await?;
                continue;
            }
        }
    }

    Ok(())
}

async fn open_connection(
    ctx: &Context,
) -> anyhow::Result<(
    impl Sink<Message, Error = WsError>,
    impl Stream<Item = Result<Message, WsError>>,
)> {
    let url = url::Url::parse(RELAY_URL)?;

    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, read) = ws_stream.split();

    let subscription_id = SubscriptionId::from_str(&Ulid::new().to_string())?;
    let mut filters = Filters::new();
    let mut filter = Filter::default();
    filter.authors = get_followings(ctx)
        .await?
        .into_iter()
        .map(HexPrefix::from)
        .collect();
    filter.since = Some(Utc::now() - chrono::Duration::seconds(5));
    filters.push(filter);

    let req = Request::Req {
        subscription_id,
        filters,
    };
    let req = serde_json::to_string(&req)?;
    let req = Message::Text(req);

    write.send(req).await?;

    Ok((write, read))
}
