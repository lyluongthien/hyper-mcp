use std::io;
use postgres_protocol::message::backend;
use crate::net::Connection;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl IsolationLevel {
    fn as_str(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED",
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

pub struct Transaction<'a> {
    connection: &'a mut Connection,
    isolation_level: IsolationLevel,
    is_active: bool,
}

impl<'a> Transaction<'a> {
    pub fn new(connection: &'a mut Connection, isolation_level: IsolationLevel) -> Self {
        Self {
            connection,
            isolation_level,
            is_active: false,
        }
    }

    pub fn begin(&mut self) -> io::Result<&[u8]> {
        if self.is_active {
            return Err(io::Error::new(io::ErrorKind::Other, "Transaction already active"));
        }

        let query = format!("BEGIN TRANSACTION ISOLATION LEVEL {}", self.isolation_level.as_str());
        let result = self.connection.prepare_query(&query)?;
        self.is_active = true;
        Ok(result)
    }

    pub fn commit(&mut self) -> io::Result<&[u8]> {
        if !self.is_active {
            return Err(io::Error::new(io::ErrorKind::Other, "No active transaction"));
        }

        let result = self.connection.prepare_query("COMMIT")?;
        self.is_active = false;
        Ok(result)
    }

    pub fn rollback(&mut self) -> io::Result<&[u8]> {
        if !self.is_active {
            return Err(io::Error::new(io::ErrorKind::Other, "No active transaction"));
        }

        let result = self.connection.prepare_query("ROLLBACK")?;
        self.is_active = false;
        Ok(result)
    }

    pub fn savepoint(&mut self, name: &str) -> io::Result<&[u8]> {
        if !self.is_active {
            return Err(io::Error::new(io::ErrorKind::Other, "No active transaction"));
        }

        let query = format!("SAVEPOINT {}", name);
        self.connection.prepare_query(&query)
    }

    pub fn rollback_to(&mut self, name: &str) -> io::Result<&[u8]> {
        if !self.is_active {
            return Err(io::Error::new(io::ErrorKind::Other, "No active transaction"));
        }

        let query = format!("ROLLBACK TO SAVEPOINT {}", name);
        self.connection.prepare_query(&query)
    }

    pub fn release(&mut self, name: &str) -> io::Result<&[u8]> {
        if !self.is_active {
            return Err(io::Error::new(io::ErrorKind::Other, "No active transaction"));
        }

        let query = format!("RELEASE SAVEPOINT {}", name);
        self.connection.prepare_query(&query)
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        if self.is_active {
            let _ = self.rollback();
        }
    }
} 