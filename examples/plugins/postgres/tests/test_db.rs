use testcontainers::{clients::Cli, Container, images::postgres::Postgres};
use tokio_postgres::{Client, NoTls};
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    static ref DOCKER: Arc<Cli> = Arc::new(Cli::default());
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct TestDb {
    container: Container<'static, Postgres>,
    connection_string: String,
    client: Option<Client>,
}

impl TestDb {
    fn new() -> Self {
        let container = DOCKER.run(Postgres::default());
        let port = container.get_host_port_ipv4(5432);
        let connection_string = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            port
        );
        
        Self {
            container,
            connection_string,
            client: None,
        }
    }

    async fn connect(&mut self) -> TestResult {
        let (client, connection) = tokio_postgres::connect(&self.connection_string, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });
        self.client = Some(client);
        Ok(())
    }

    async fn setup_test_table(&mut self) -> TestResult {
        let client = self.client.as_mut().unwrap();
        client.execute(
            "CREATE TABLE IF NOT EXISTS test_transactions (id SERIAL PRIMARY KEY, value TEXT)",
            &[],
        ).await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_db_connection() -> TestResult {
    let mut test_db = TestDb::new();
    test_db.connect().await
}

#[tokio::test]
async fn test_db_query() -> TestResult {
    let mut test_db = TestDb::new();
    test_db.connect().await?;
    let client = test_db.client.as_ref().unwrap();
    
    let rows = client.query("SELECT 1 as test", &[]).await?;
    assert_eq!(rows[0].get::<_, i32>("test"), 1);
    Ok(())
}

#[tokio::test]
async fn test_transaction_commit() -> TestResult {
    let mut test_db = TestDb::new();
    test_db.connect().await?;
    test_db.setup_test_table().await?;
    let client = test_db.client.as_mut().unwrap();

    let tx = client.transaction().await?;
    tx.execute(
        "INSERT INTO test_transactions (value) VALUES ($1)",
        &[&"test_value"],
    ).await?;
    tx.commit().await?;

    let rows = client
        .query("SELECT value FROM test_transactions", &[])
        .await?;
    assert_eq!(rows[0].get::<_, String>("value"), "test_value");
    Ok(())
}

#[tokio::test]
async fn test_transaction_rollback() -> TestResult {
    let mut test_db = TestDb::new();
    test_db.connect().await?;
    test_db.setup_test_table().await?;
    let client = test_db.client.as_mut().unwrap();

    let tx = client.transaction().await?;
    tx.execute(
        "INSERT INTO test_transactions (value) VALUES ($1)",
        &[&"rollback_value"],
    ).await?;
    tx.rollback().await?;

    let rows = client
        .query("SELECT value FROM test_transactions", &[])
        .await?;
    assert_eq!(rows.len(), 0);
    Ok(())
} 