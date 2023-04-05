use nostr_core::RawEvent;
use tokio::sync::broadcast::{error::RecvError, Sender};

use crate::Connections;

#[derive(Clone)]
pub struct Context {
    pub connections: Connections,
    pub event_broadcast: Sender<RawEvent>,
    pub dynamodb: aws_sdk_dynamodb::Client,
}

impl Context {
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;
        let (event_broadcast, mut rx) = tokio::sync::broadcast::channel(1000);

        // プロセスが生きてる間、受信側を常に起動しておく
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Err(RecvError::Closed) => break,
                    _ => {} // noop
                }
            }
        });

        Self {
            connections: Connections::new(),
            event_broadcast,
            dynamodb: aws_sdk_dynamodb::Client::new(&config),
        }
    }
}
