use std::collections::HashMap;

use aws_sdk_dynamodb as dynamodb;
use dynamodb::model::{AttributeValue, DeleteRequest, WriteRequest};

use crate::util;

pub async fn run() -> anyhow::Result<()> {
    let dynamodb = util::dynamodb().await;

    reset_table(&dynamodb, "events", &["id"]).await?;

    Ok(())
}

async fn reset_table(
    client: &dynamodb::Client,
    table_name: &str,
    keys: &[&str],
) -> anyhow::Result<()> {
    let mut exclusive_start_keys: HashMap<String, AttributeValue> = Default::default();
    let mut write_requests: Vec<WriteRequest> = Default::default();

    loop {
        let mut scan = client.scan().table_name(table_name);
        for key in keys {
            scan = scan.attributes_to_get(*key);
        }
        for (k, v) in exclusive_start_keys.iter() {
            scan = scan.exclusive_start_key(k, v.clone());
        }
        let result = scan.send().await?;
        let items = result.items().unwrap_or_default();

        for item in items {
            let mut delete_request = DeleteRequest::builder();
            for key in keys {
                delete_request = delete_request.key(*key, item.get(*key).unwrap().clone());
            }
            let write_request = WriteRequest::builder()
                .delete_request(delete_request.build())
                .build();
            write_requests.push(write_request);

            if write_requests.len() >= 25 {
                client
                    .batch_write_item()
                    .request_items(table_name, write_requests.clone())
                    .send()
                    .await?;
                write_requests.clear();
            }
        }

        let Some(last_evaluated_key) = result.last_evaluated_key() else { break };
        exclusive_start_keys = last_evaluated_key.clone();
    }

    if !write_requests.is_empty() {
        client
            .batch_write_item()
            .request_items(table_name, write_requests)
            .send()
            .await?;
    }

    Ok(())
}
