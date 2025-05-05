mod net;
mod error;
mod pdk;
mod transaction;

pub use error::Error;
pub use net::wasm::WasmSocket;
pub use transaction::{Transaction, IsolationLevel};

use extism_pdk::*;
use pdk::types::{CallToolRequest, CallToolResult, Content, ContentType, ListToolsResult, ToolDescription};
use serde_json::json;
use std::sync::Once;
use bytes::{BytesMut, BufMut};
use postgres_protocol::message::frontend;
use postgres_protocol::message::backend;

static DB_INIT: Once = Once::new();
static mut BUFFER: Option<BytesMut> = None;

fn get_db_url() -> Result<String, Error> {
    config::get("database_url")?
        .ok_or_else(|| Error::msg("database_url configuration is required but not set"))
}

fn init_db(db_url: &str) -> Result<(), Error> {
    let mut buffer = BytesMut::with_capacity(1024);
    frontend::startup_message(vec![("user", "postgres"), ("database", "postgres")], &mut buffer)?;
    unsafe {
        BUFFER = Some(buffer);
    }
    Ok(())
}

fn execute_read_query(query: &str) -> Result<String, Error> {
    let buffer = unsafe { BUFFER.as_mut() }
        .ok_or_else(|| Error::msg("Database not initialized"))?;

    buffer.clear();
    frontend::query(query, buffer)?;

    // In a real implementation, we would send this buffer over a socket
    // and parse the response. For now, we'll return a mock response
    Ok(json!([{"mock": "data"}]).to_string())
}

fn execute_write_query(query: &str) -> Result<String, Error> {
    let buffer = unsafe { BUFFER.as_mut() }
        .ok_or_else(|| Error::msg("Database not initialized"))?;

    buffer.clear();
    frontend::query(query, buffer)?;

    // Mock response
    Ok(json!({"rows_affected": 1}).to_string())
}

fn list_tables() -> Result<String, Error> {
    execute_read_query("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'")
}

fn describe_table(table_name: &str) -> Result<String, Error> {
    execute_read_query(&format!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = '{}' 
         AND table_schema = 'public'",
        table_name
    ))
}

pub(crate) fn call(input: CallToolRequest) -> Result<CallToolResult, Error> {
    let db_url = get_db_url()?;
    DB_INIT.call_once(|| {
        init_db(&db_url).expect("Failed to initialize database");
    });

    match input.params.name.as_str() {
        "postgres_read_query" => {
            let args = input.params.arguments.unwrap_or_default();
            let query = match args.get("query") {
                Some(v) if v.is_string() => v.as_str().unwrap(),
                _ => return Err(Error::msg("query parameter is required")),
            };

            let result = execute_read_query(query)?;

            Ok(CallToolResult {
                is_error: None,
                content: vec![Content {
                    annotations: None,
                    text: Some(result),
                    mime_type: Some("application/json".to_string()),
                    r#type: ContentType::Text,
                    data: None,
                }],
            })
        }
        "postgres_write_query" => {
            let args = input.params.arguments.unwrap_or_default();
            let query = match args.get("query") {
                Some(v) if v.is_string() => v.as_str().unwrap(),
                _ => return Err(Error::msg("query parameter is required")),
            };

            let result = execute_write_query(query)?;

            Ok(CallToolResult {
                is_error: None,
                content: vec![Content {
                    annotations: None,
                    text: Some(result),
                    mime_type: Some("application/json".to_string()),
                    r#type: ContentType::Text,
                    data: None,
                }],
            })
        }
        "postgres_list_tables" => {
            let result = list_tables()?;

            Ok(CallToolResult {
                is_error: None,
                content: vec![Content {
                    annotations: None,
                    text: Some(result),
                    mime_type: Some("application/json".to_string()),
                    r#type: ContentType::Text,
                    data: None,
                }],
            })
        }
        "postgres_describe_table" => {
            let args = input.params.arguments.unwrap_or_default();
            let table_name = match args.get("table_name") {
                Some(v) if v.is_string() => v.as_str().unwrap(),
                _ => return Err(Error::msg("table_name parameter is required")),
            };

            let result = describe_table(table_name)?;

            Ok(CallToolResult {
                is_error: None,
                content: vec![Content {
                    annotations: None,
                    text: Some(result),
                    mime_type: Some("application/json".to_string()),
                    r#type: ContentType::Text,
                    data: None,
                }],
            })
        }
        _ => Ok(CallToolResult {
            is_error: Some(true),
            content: vec![Content {
                annotations: None,
                text: Some(format!("Unknown tool: {}", input.params.name)),
                mime_type: None,
                r#type: ContentType::Text,
                data: None,
            }],
        }),
    }
}

pub(crate) fn describe() -> Result<ListToolsResult, Error> {
    Ok(ListToolsResult {
        tools: vec![
            ToolDescription {
                name: "postgres_read_query".into(),
                description: "Execute a SELECT query on the PostgreSQL database".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "SELECT SQL query to execute",
                        }
                    },
                    "required": ["query"],
                })
                .as_object()
                .unwrap()
                .clone(),
            },
            ToolDescription {
                name: "postgres_write_query".into(),
                description: "Execute an INSERT, UPDATE, or DELETE query on the PostgreSQL database"
                    .into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "SQL query to execute",
                        }
                    },
                    "required": ["query"],
                })
                .as_object()
                .unwrap()
                .clone(),
            },
            ToolDescription {
                name: "postgres_list_tables".into(),
                description: "List all tables in the PostgreSQL database".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": [],
                })
                .as_object()
                .unwrap()
                .clone(),
            },
            ToolDescription {
                name: "postgres_describe_table".into(),
                description: "Get the schema information for a specific table".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "table_name": {
                            "type": "string",
                            "description": "Name of the table to describe",
                        }
                    },
                    "required": ["table_name"],
                })
                .as_object()
                .unwrap()
                .clone(),
            },
        ],
    })
}
 