# PostgreSQL Plugin for hyper-mcp

A WASM-compatible PostgreSQL plugin that implements the PostgreSQL wire protocol using WASI preview2 for network communication.

## Features

- Full PostgreSQL wire protocol support
- WASM compatibility using WASI preview2
- Transaction support with all isolation levels
- Save point management
- Automatic transaction rollback on drop
- Comprehensive error handling
- Docker-based integration testing

## Installation

Add the plugin to your `Cargo.toml`:

```toml
[dependencies]
postgres-plugin = { git = "https://github.com/your-org/hyper-mcp" }
```

## Usage

### Basic Connection

```rust
use postgres_plugin::WasmSocket;

let mut socket = WasmSocket::new();
socket.connect("127.0.0.1:5432").expect("Failed to connect");

let result = socket.execute_query("SELECT version()");
```

### Transaction Management

```rust
use postgres_plugin::{WasmSocket, IsolationLevel};

let mut socket = WasmSocket::new();
socket.connect("127.0.0.1:5432").expect("Failed to connect");

// Begin a transaction with specified isolation level
let mut tx = socket.begin_transaction(IsolationLevel::ReadCommitted)
    .expect("Failed to begin transaction");

// Execute queries within transaction
socket.execute_query("CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)");
socket.execute_query("INSERT INTO users (name) VALUES ('test')");

// Create a save point
tx.savepoint("before_update").expect("Failed to create save point");

// Execute more queries
socket.execute_query("UPDATE users SET name = 'updated'");

// Rollback to save point if needed
tx.rollback_to("before_update").expect("Failed to rollback to save point");

// Commit or rollback
tx.commit().expect("Failed to commit transaction");
// Or tx.rollback() to discard changes
```

### Isolation Levels

The following isolation levels are supported:

- `ReadUncommitted`
- `ReadCommitted`
- `RepeatableRead`
- `Serializable`

### Error Handling

The plugin provides comprehensive error handling:

```rust
use postgres_plugin::Error;

match socket.execute_query("SELECT * FROM non_existent_table") {
    Ok(result) => {
        // Process result
    }
    Err(Error::Other(msg)) => {
        eprintln!("Query failed: {}", msg);
    }
}
```

## Development

### Prerequisites

- Rust toolchain with wasm32-wasi target
- Docker for integration tests
- WASI preview2 SDK

### Building

```bash
cargo build --target wasm32-wasi
```

### Testing

Run the test suite:

```bash
cargo test
```

For integration tests with Docker:

```bash
cargo test --features integration-tests
```

## Technical Details

### WASI preview2 Socket Implementation

The plugin uses WASI preview2's networking capabilities for socket communication:

- TCP socket creation and management
- Asynchronous I/O operations
- Error handling and recovery
- Connection pooling (planned)

### PostgreSQL Wire Protocol

Implements the core PostgreSQL wire protocol messages:

- Startup/Authentication
- Query execution
- Transaction management
- Parameter handling
- Result set processing

### Transaction Management

The plugin provides a robust transaction management system:

- ACID compliance through PostgreSQL
- Automatic rollback on drop (RAII)
- Save point support
- All standard isolation levels

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit pull requests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
