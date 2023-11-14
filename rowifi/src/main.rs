use std::error::Error;
use rowifi_database::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let connection_string = std::env::var("DATABASE_CONN").expect("expected a database connection string.");

    let database = Database::new(&connection_string).await;

    Ok(())
}
