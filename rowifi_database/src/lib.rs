use aws_sdk_dynamodb::Client as DynamoClient;

pub use crate::error::DatabaseError;

mod error;

pub struct Database {
    client: DynamoClient,
}

impl Database {
    pub async fn new() -> Self {
       let config = aws_config::load_from_env().await;
       let client = DynamoClient::new(&config);

       Self { client }
    }
}
