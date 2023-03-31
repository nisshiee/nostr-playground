use crate::Connections;

#[derive(Clone)]
pub struct Context {
    pub connections: Connections,
    pub dynamodb: aws_sdk_dynamodb::Client,
}

impl Context {
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;
        Self {
            connections: Connections::new(),
            dynamodb: aws_sdk_dynamodb::Client::new(&config),
        }
    }
}
