use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] tokio_postgres::Error),

    #[error("Query execution error: {0}")]
    QueryError(#[from] tokio_postgres::Error),

    #[error("Transaction error: {0}")]
    TransactionError(#[from] tokio_postgres::Error),

    #[error("Not connected to database")]
    NotConnected,

    #[error("Configuration error: {0}")]
    ConfigError(String),
} 