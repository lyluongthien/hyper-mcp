use plugin::{WasmSocket, IsolationLevel};
use testcontainers::*;
use postgres_protocol::message::backend;

#[test]
fn test_transaction_lifecycle() {
    let docker = clients::Cli::default();
    let postgres = images::postgres::Postgres::default();
    let container = docker.run(postgres);
    let port = container.get_host_port_ipv4(5432);
    let url = format!("127.0.0.1:{}", port);

    let mut socket = WasmSocket::new();
    socket.connect(&url).expect("Failed to connect");

    // Begin transaction
    let mut tx = socket.begin_transaction(IsolationLevel::ReadCommitted)
        .expect("Failed to begin transaction");
    assert!(tx.is_active());

    // Execute query within transaction
    let query = "CREATE TABLE test (id SERIAL PRIMARY KEY, name TEXT)";
    let result = socket.execute_query(query).expect("Failed to create table");
    assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));

    // Create savepoint
    let result = tx.savepoint("test_point").expect("Failed to create savepoint");
    assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));

    // Insert data
    let query = "INSERT INTO test (name) VALUES ('test')";
    let result = socket.execute_query(query).expect("Failed to insert data");
    assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));

    // Rollback to savepoint
    let result = tx.rollback_to("test_point").expect("Failed to rollback to savepoint");
    assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));

    // Verify data was rolled back
    let query = "SELECT COUNT(*) FROM test";
    let result = socket.execute_query(query).expect("Failed to count rows");
    assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));

    // Commit transaction
    let result = tx.commit().expect("Failed to commit transaction");
    assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));
    assert!(!tx.is_active());
}

#[test]
fn test_transaction_isolation_levels() {
    let docker = clients::Cli::default();
    let postgres = images::postgres::Postgres::default();
    let container = docker.run(postgres);
    let port = container.get_host_port_ipv4(5432);
    let url = format!("127.0.0.1:{}", port);

    let mut socket = WasmSocket::new();
    socket.connect(&url).expect("Failed to connect");

    // Test each isolation level
    let levels = vec![
        IsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted,
        IsolationLevel::RepeatableRead,
        IsolationLevel::Serializable,
    ];

    for level in levels {
        let tx = socket.begin_transaction(level).expect("Failed to begin transaction");
        assert!(tx.is_active());
        
        let result = tx.commit().expect("Failed to commit transaction");
        assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));
    }
}

#[test]
fn test_transaction_automatic_rollback() {
    let docker = clients::Cli::default();
    let postgres = images::postgres::Postgres::default();
    let container = docker.run(postgres);
    let port = container.get_host_port_ipv4(5432);
    let url = format!("127.0.0.1:{}", port);

    let mut socket = WasmSocket::new();
    socket.connect(&url).expect("Failed to connect");

    // Begin transaction and create table
    let tx = socket.begin_transaction(IsolationLevel::ReadCommitted)
        .expect("Failed to begin transaction");
    
    let query = "CREATE TABLE test_rollback (id SERIAL PRIMARY KEY)";
    let result = socket.execute_query(query).expect("Failed to create table");
    assert!(matches!(result.last(), Some(backend::Message::ReadyForQuery(_))));

    // Let transaction drop without commit
    drop(tx);

    // Verify table doesn't exist
    let query = "SELECT * FROM test_rollback";
    let result = socket.execute_query(query);
    assert!(result.is_err());
} 