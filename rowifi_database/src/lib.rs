#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use deadpool_postgres::{Manager, Object, Pool, Runtime};
use std::{str::FromStr, time::Duration};
use tokio_postgres::types::ToSql;
use tokio_postgres::{Config as TokioPostgresConfig, NoTls, Row};

pub use crate::error::DatabaseError;
pub use tokio_postgres as postgres;
pub use deadpool_postgres as deadpool;

mod error;

#[derive(Clone)]
pub struct Database {
    pool: Pool,
}

impl Database {
    /// Create a connection pool to the database with the given connection string
    ///
    /// # Panics
    /// May panic if the pool is built incorrectly.
    pub async fn new(connection_string: &str) -> Self {
        let postgres_config = TokioPostgresConfig::from_str(connection_string).unwrap();
        let manager = Manager::new(postgres_config, NoTls);
        let pool = Pool::builder(manager)
            .max_size(16)
            .runtime(Runtime::Tokio1)
            .recycle_timeout(Some(Duration::from_secs(30)))
            .create_timeout(Some(Duration::from_secs(30)))
            .wait_timeout(Some(Duration::from_secs(30)))
            .build()
            .unwrap();

        tracing::info!("attempting database connection");
        let _ = pool.get().await.unwrap();

        tracing::info!("database connection successful");
        Self { pool }
    }

    /// Get a connection from the pool
    /// # Errors
    ///
    /// See [`PoolError`] for details.
    #[inline]
    pub async fn get(&self) -> Result<Object, DatabaseError> {
        let conn = self.pool.get().await?;
        Ok(conn)
    }

    /// Get a list of items from a query. This functions converts the statement query into a prepared statement and caches it.
    /// # Errors
    ///
    /// Return Err if there is a pool error or the query failed to acquire data from the database
    /// or the returned data could not be deserialized.
    pub async fn query<T>(
        &self,
        statement: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<T>, DatabaseError>
    where
        T: TryFrom<Row>,
        DatabaseError: From<<T as TryFrom<Row>>::Error>,
    {
        let conn = self.get().await?;
        let statement = conn.prepare_cached(statement).await?;
        let rows = conn.query(&statement, params).await?;
        let items = rows
            .into_iter()
            .map(|r| T::try_from(r))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }

    /// Get an item from a query. Returns [None] if the item does not exist.
    /// # Errors
    ///
    /// Return Err if there is a pool error or the query failed to acquire data from the database
    /// or the returned data could not be deserialized.
    pub async fn query_opt<T>(
        &self,
        statement: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<T>, DatabaseError>
    where
        T: TryFrom<Row>,
        DatabaseError: From<<T as TryFrom<Row>>::Error>,
    {
        let conn = self.get().await?;
        let statement = conn.prepare_cached(statement).await?;
        let row = conn.query_opt(&statement, params).await?;
        match row {
            Some(r) => Ok(Some(T::try_from(r)?)),
            None => Ok(None),
        }
    }

    /// Execute the given query. Returns the number of rows modified.
    /// # Errors
    ///
    /// Return Err if there is a pool error or the query failed to acquire data from the database
    /// or the returned data could not be deserialized.
    pub async fn execute(
        &self,
        statement: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, DatabaseError> {
        let conn = self.get().await?;
        let statement = conn.prepare_cached(statement).await?;
        let row = conn.execute(&statement, params).await?;
        Ok(row)
    }
}
