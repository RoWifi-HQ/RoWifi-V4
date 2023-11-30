use aws_config::Region;
use aws_sdk_dynamodb::Client as DynamoClient;

pub use crate::error::DatabaseError;
pub use aws_sdk_dynamodb;

mod error;

pub struct Database {
    pub client: DynamoClient,
}

impl Database {
    pub async fn new() -> Self {
        let region = Region::from_static("us-west-2");
        let config = aws_config::from_env().region(region).load().await;
        let client = DynamoClient::new(&config);

        Self { client }
    }
}
