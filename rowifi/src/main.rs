use std::error::Error;
use rowifi_database::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let database = Database::new().await;

    Ok(())
}
