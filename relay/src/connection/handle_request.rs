use std::{collections::HashMap, time::Duration};

use aws_sdk_dynamodb::model::AttributeValue;
use nostr_core::{Filters, RawEvent, Request, Response, SubscriptionId};
use serde_dynamo::to_attribute_value;
use tokio::sync::broadcast::{error::RecvError, Receiver};
use tokio_tungstenite::tungstenite::Message;
use ulid::Ulid;

use crate::{connection::Status, Connection, Context, Query};

#[tracing::instrument(skip_all, fields(r#type = req.type_str()))]
pub async fn handle_request(ctx: Context, conn: Connection, req: Request) -> anyhow::Result<()> {
    tracing::info!("{req:?}");

    match req {
        Request::Event(event) => {
            if !event.verify() {
                tracing::info!("verify failed");
                return Ok(());
            }
            ctx.event_broadcaster.send(event.clone());
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
            let response = Response::Eose(subscription_id.clone());
            let message = Message::Text(serde_json::to_string(&response)?);
            conn.send_raw(message);
            if let Status::Connected { subscriptions } = &mut *conn.status.lock().await {
                let ulid = Ulid::new();
                subscriptions.insert(subscription_id.clone(), ulid);
                tracing::info!("{:?}", subscriptions);

                let conn = conn.clone();
                let rx = ctx.event_broadcaster.subscribe();
                tokio::spawn(new_subscription(conn, subscription_id, ulid, rx, filters));
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

#[tracing::instrument(name = "subscription", skip(conn, rx, filters), fields(addr = conn.addr().to_string()))]
async fn new_subscription(
    conn: Connection,
    subscription_id: SubscriptionId,
    ulid: Ulid,
    mut rx: Receiver<RawEvent>,
    filters: Filters,
) {
    tracing::info!(?filters, "filters");
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
                        let Status::Connected { subscriptions } = &*conn.status.lock().await else { break; };
                        let Some(existing_ulid) = subscriptions.get(&subscription_id) else { break; };
                        if existing_ulid != &ulid {
                            break;
                        }
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
}
