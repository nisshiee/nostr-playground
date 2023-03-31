use aws_sdk_dynamodb as dynamodb;

pub async fn dynamodb() -> dynamodb::Client {
    let config = aws_config::load_from_env().await;
    dynamodb::Client::new(&config)
}
