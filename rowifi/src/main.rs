use rowifi_database::Database;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let database = Database::new().await;

    Ok(())
}
