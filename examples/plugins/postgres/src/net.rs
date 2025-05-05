use std::io::{self, Read, Write};
use bytes::{BytesMut, BufMut};
use postgres_protocol::message::{frontend, backend};
use wasi_preview2::*;
use wasi_cap_std_sync::net::{TcpSocket, SocketAddr};
use crate::transaction::{Transaction, IsolationLevel};

pub struct Connection {
    buffer: BytesMut,
    response: BytesMut,
}

impl Connection {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(1024),
            response: BytesMut::with_capacity(1024),
        }
    }

    pub fn prepare_startup(&mut self, params: &[(&str, &str)]) -> io::Result<&[u8]> {
        self.buffer.clear();
        frontend::startup_message(params, &mut self.buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(&self.buffer)
    }

    pub fn prepare_query(&mut self, query: &str) -> io::Result<&[u8]> {
        self.buffer.clear();
        frontend::query(query, &mut self.buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(&self.buffer)
    }

    pub fn parse_response(&mut self, data: &[u8]) -> io::Result<Vec<backend::Message>> {
        self.response.clear();
        self.response.extend_from_slice(data);
        
        let mut messages = Vec::new();
        let mut offset = 0;

        while offset < self.response.len() {
            match backend::Message::parse(&self.response[offset..]) {
                Ok((message, bytes_consumed)) => {
                    messages.push(message);
                    offset += bytes_consumed;
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, e));
                }
            }
        }

        Ok(messages)
    }
}

#[cfg(target_family = "wasm")]
pub mod wasm {
    use super::*;
    use extism_pdk::*;
    use std::str::FromStr;

    pub struct WasmSocket {
        connection: Connection,
        socket: Option<TcpSocket>,
    }

    impl WasmSocket {
        pub fn new() -> Self {
            Self {
                connection: Connection::new(),
                socket: None,
            }
        }

        pub fn begin_transaction(&mut self, isolation_level: IsolationLevel) -> Result<Transaction, Error> {
            let mut tx = Transaction::new(&mut self.connection, isolation_level);
            let begin_msg = tx.begin()
                .map_err(|e| Error::Other(format!("Failed to begin transaction: {}", e)))?;

            let socket = self.socket.as_mut()
                .ok_or_else(|| Error::Other("Not connected".into()))?;

            socket.write_all(begin_msg)
                .map_err(|e| Error::Other(format!("Failed to send begin transaction: {}", e)))?;

            let mut response = vec![0; 1024];
            let n = socket.read(&mut response)
                .map_err(|e| Error::Other(format!("Failed to read response: {}", e)))?;

            let messages = self.connection.parse_response(&response[..n])?;
            
            for msg in messages {
                match msg {
                    backend::Message::ErrorResponse(_) => {
                        return Err(Error::Other("Failed to begin transaction".into()));
                    }
                    backend::Message::ReadyForQuery(_) => {
                        return Ok(tx);
                    }
                    _ => continue,
                }
            }

            Err(Error::Other("Unexpected server response".into()))
        }

        pub fn connect(&mut self, url: &str) -> Result<(), Error> {
            let addr = SocketAddr::from_str(url)
                .map_err(|e| Error::Other(format!("Invalid address: {}", e)))?;

            let socket = TcpSocket::new()
                .map_err(|e| Error::Other(format!("Failed to create socket: {}", e)))?;

            socket.connect(&addr)
                .map_err(|e| Error::Other(format!("Failed to connect: {}", e)))?;

            let params = vec![("user", "postgres"), ("database", "postgres")];
            let startup_msg = self.connection.prepare_startup(&params)?;

            socket.write_all(startup_msg)
                .map_err(|e| Error::Other(format!("Failed to send startup message: {}", e)))?;

            let mut response = vec![0; 1024];
            let n = socket.read(&mut response)
                .map_err(|e| Error::Other(format!("Failed to read response: {}", e)))?;

            let messages = self.connection.parse_response(&response[..n])?;
            
            for msg in messages {
                match msg {
                    backend::Message::ErrorResponse(_) => {
                        return Err(Error::Other("Authentication failed".into()));
                    }
                    backend::Message::AuthenticationOk => {
                        self.socket = Some(socket);
                        return Ok(());
                    }
                    _ => continue,
                }
            }

            Err(Error::Other("Unexpected server response".into()))
        }

        pub fn execute_query(&mut self, query: &str) -> Result<Vec<backend::Message>, Error> {
            let socket = self.socket.as_mut()
                .ok_or_else(|| Error::Other("Not connected".into()))?;

            let query_msg = self.connection.prepare_query(query)?;
            socket.write_all(query_msg)
                .map_err(|e| Error::Other(format!("Failed to send query: {}", e)))?;

            let mut response = vec![0; 1024];
            let n = socket.read(&mut response)
                .map_err(|e| Error::Other(format!("Failed to read response: {}", e)))?;

            self.connection.parse_response(&response[..n])
                .map_err(|e| Error::Other(format!("Failed to parse response: {}", e)))
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub mod native {
    use super::*;
    use std::net::TcpStream;

    pub struct NativeSocket {
        stream: TcpStream,
        connection: Connection,
    }

    impl NativeSocket {
        pub fn new(stream: TcpStream) -> Self {
            Self {
                stream,
                connection: Connection::new(),
            }
        }

        pub fn connect(addr: &str) -> io::Result<Self> {
            let stream = TcpStream::connect(addr)?;
            Ok(Self::new(stream))
        }

        pub fn execute_query(&mut self, query: &str) -> io::Result<Vec<backend::Message>> {
            let query_msg = self.connection.prepare_query(query)?;
            self.stream.write_all(query_msg)?;

            let mut response = vec![0; 1024];
            let n = self.stream.read(&mut response)?;
            self.connection.parse_response(&response[..n])
        }
    }
} 