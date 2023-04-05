use aws_sdk_dynamodb::model::AttributeValue;
use chrono::{DateTime, Utc};
use nostr_core::RawEvent;
use serde::Serialize;

use crate::Context;

#[derive(Clone, Debug, Default)]
pub struct Query {
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

impl Query {
    pub fn new(since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>) -> Self {
        Self { since, until }
    }

    pub async fn exec(&self, ctx: Context) -> anyhow::Result<Vec<RawEvent>> {
        let since = self.since.map(|t| t.timestamp()).unwrap_or(0);
        let until = self.until.map(|t| t.timestamp()).unwrap_or(i64::MAX);
        let output = ctx
            .dynamodb
            .scan()
            .table_name("events")
            .filter_expression("created_at BETWEEN :since AND :until")
            .expression_attribute_values(":since", to_attribute_value(since))
            .expression_attribute_values(":until", to_attribute_value(until))
            .send()
            .await?;

        let mut ret = Vec::with_capacity(output.count() as usize);
        for item in output.items.unwrap_or_default() {
            let event: RawEvent = serde_dynamo::from_item(item)?;
            ret.push(event);
        }
        ret.sort_by_key(|event| event.created_at);
        ret.reverse();
        Ok(ret)
    }
}

fn to_attribute_value<T>(value: T) -> AttributeValue
where
    T: Serialize,
{
    serde_dynamo::to_attribute_value(value).expect("AttributeValueへの型変換失敗Logic failuer")
}
