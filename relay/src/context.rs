use crate::Connections;

mod event_broadcaster;
use event_broadcaster::EventBroadcaster;

#[derive(Clone)]
pub struct Context {
    pub connections: Connections,
    pub event_broadcaster: EventBroadcaster,
    pub dynamodb: aws_sdk_dynamodb::Client,
}

impl Context {
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;

        Self {
            connections: Connections::new(),
            event_broadcaster: EventBroadcaster::new(),
            dynamodb: aws_sdk_dynamodb::Client::new(&config),
        }
    }
}
