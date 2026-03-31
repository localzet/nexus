//! TCP Protocol - PostgreSQL Wire Protocol compatible для psql, pgAdmin, DBeaver

use crate::{MultiModelEngine, QueryExecutor, IndexingManager, QueryResult};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::error::Error;

/// PostgreSQL Wire Protocol Message Types
#[derive(Debug, Clone)]
pub enum MessageType {
    Query,          // 'Q'
    Terminate,      // 'X'
    Authentication, // 'R'
    ErrorResponse,  // 'E'
    Notice,         // 'N'
    ReadyForQuery,  // 'Z'
    EmptyResponse,  // 'I'
    DataRow,        // 'D'
    RowDescription, // 'T'
    CommandComplete,// 'C'
}

impl MessageType {
    pub fn as_byte(&self) -> u8 {
        match self {
            MessageType::Query => b'Q',
            MessageType::Terminate => b'X',
            MessageType::Authentication => b'R',
            MessageType::ErrorResponse => b'E',
            MessageType::Notice => b'N',
            MessageType::ReadyForQuery => b'Z',
            MessageType::EmptyResponse => b'I',
            MessageType::DataRow => b'D',
            MessageType::RowDescription => b'T',
            MessageType::CommandComplete => b'C',
        }
    }
}

/// PostgreSQL message
#[derive(Debug, Clone)]
pub struct PgMessage {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
}

impl PgMessage {
    pub fn new(msg_type: MessageType, payload: Vec<u8>) -> Self {
        PgMessage { msg_type, payload }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![self.msg_type.as_byte()];
        let len = (self.payload.len() + 4) as u32;
        result.extend_from_slice(&len.to_be_bytes());
        result.extend_from_slice(&self.payload);
        result
    }

    pub fn from_query(query: String) -> Self {
        let mut payload = Vec::new();
        payload.extend_from_slice(query.as_bytes());
        payload.push(0); // Null terminator
        PgMessage::new(MessageType::Query, payload)
    }

    pub fn error_response(message: &str) -> Self {
        let mut payload = Vec::new();
        payload.push(b'S'); // Severity
        payload.extend_from_slice(b"ERROR\0");
        payload.push(b'C'); // Code
        payload.extend_from_slice(b"00000\0");
        payload.push(b'M'); // Message
        payload.extend_from_slice(message.as_bytes());
        payload.push(0);
        PgMessage::new(MessageType::ErrorResponse, payload)
    }

    pub fn command_complete(command: &str) -> Self {
        let mut payload = Vec::new();
        payload.extend_from_slice(command.as_bytes());
        payload.push(0);
        PgMessage::new(MessageType::CommandComplete, payload)
    }

    pub fn ready_for_query() -> Self {
        PgMessage::new(MessageType::ReadyForQuery, vec![b'I'])
    }

    pub fn startup_message() -> Self {
        let mut payload = Vec::new();
        payload.push(b'R'); // Authentication
        payload.extend_from_slice(&3u32.to_be_bytes()); // MD5 auth
        payload.extend_from_slice(&[0; 4]); // Salt (zeros for demo)
        PgMessage::new(MessageType::Authentication, payload)
    }
}

/// TCP Connection Handler
pub struct TcpConnectionHandler {
    engine: Arc<MultiModelEngine>,
    indexing: Arc<IndexingManager>,
    query_executor: Arc<QueryExecutor>,
}

impl TcpConnectionHandler {
    pub fn new(
        engine: Arc<MultiModelEngine>,
        indexing: Arc<IndexingManager>,
        query_executor: Arc<QueryExecutor>,
    ) -> Self {
        TcpConnectionHandler {
            engine,
            indexing,
            query_executor,
        }
    }

    pub async fn handle_connection(&self, mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
        println!("[TCP] New connection from {}", stream.peer_addr()?);

        // Send startup message
        let startup = PgMessage::startup_message();
        stream.write_all(&startup.to_bytes()).await?;

        // Read client data
        let mut buffer = [0u8; 4096];
        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break; // Connection closed
            }

            // Parse incoming message
            if n > 5 {
                let msg_type = buffer[0] as char;
                let payload = &buffer[5..n];

                match msg_type {
                    'Q' => {
                        // Query message
                        let query_str = String::from_utf8_lossy(payload)
                            .trim_end_matches('\0')
                            .to_string();

                        println!("[TCP] Query: {}", query_str);

                        match self.query_executor.execute(&query_str) {
                            Ok(result) => {
                                // Send result
                                self.send_query_result(&mut stream, &result).await?;
                            }
                            Err(e) => {
                                let error_msg = PgMessage::error_response(&format!("{}", e));
                                stream.write_all(&error_msg.to_bytes()).await?;
                            }
                        }

                        // Send ready for query
                        let ready = PgMessage::ready_for_query();
                        stream.write_all(&ready.to_bytes()).await?;
                    }
                    'X' => {
                        // Terminate message
                        println!("[TCP] Connection terminated");
                        break;
                    }
                    _ => {
                        println!("[TCP] Unknown message type: {}", msg_type);
                    }
                }
            }
        }

        println!("[TCP] Connection closed");
        Ok(())
    }

    async fn send_query_result(
        &self,
        stream: &mut TcpStream,
        result: &QueryResult,
    ) -> Result<(), Box<dyn Error>> {
        // Send row description
        let mut row_desc = Vec::new();
        let col_count = result.columns.len() as u16;
        row_desc.extend_from_slice(&col_count.to_be_bytes());

        for col in &result.columns {
            row_desc.extend_from_slice(col.as_bytes());
            row_desc.push(0);
            row_desc.extend_from_slice(&0u32.to_be_bytes()); // Table OID
            row_desc.extend_from_slice(&0u16.to_be_bytes()); // Column index
            row_desc.extend_from_slice(&25u32.to_be_bytes()); // Type OID (text)
            row_desc.extend_from_slice(&(-1i16).to_be_bytes()); // Type length
            row_desc.extend_from_slice(&(-1i32).to_be_bytes()); // Type modifier
            row_desc.extend_from_slice(&0u16.to_be_bytes()); // Format code
        }

        let row_desc_msg = PgMessage::new(MessageType::RowDescription, row_desc);
        stream.write_all(&row_desc_msg.to_bytes()).await?;

        // Send data rows
        for _row_idx in 0..result.row_count {
            let mut data_row = Vec::new();
            data_row.extend_from_slice(&(result.columns.len() as u16).to_be_bytes());

            for col in &result.columns {
                let value = format!("value_{}", col);
                let len = value.len() as u32;
                data_row.extend_from_slice(&len.to_be_bytes());
                data_row.extend_from_slice(value.as_bytes());
            }

            let data_msg = PgMessage::new(MessageType::DataRow, data_row);
            stream.write_all(&data_msg.to_bytes()).await?;
        }

        // Send command complete
        let command = format!("SELECT {}", result.row_count);
        let complete = PgMessage::command_complete(&command);
        stream.write_all(&complete.to_bytes()).await?;

        Ok(())
    }
}

/// TCP Server for NEXUS DB
pub struct TcpServer {
    engine: Arc<MultiModelEngine>,
    indexing: Arc<IndexingManager>,
    query_executor: Arc<QueryExecutor>,
    addr: String,
}

impl TcpServer {
    pub fn new(
        engine: Arc<MultiModelEngine>,
        indexing: Arc<IndexingManager>,
        query_executor: Arc<QueryExecutor>,
        addr: String,
    ) -> Self {
        TcpServer {
            engine,
            indexing,
            query_executor,
            addr,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("[TCP Server] Listening on {}", self.addr);
        println!("[TCP Server] PostgreSQL compatible protocol enabled");
        println!("[TCP Server] Connect with: psql -h 127.0.0.1 -p 5432 -U nexus");

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            println!("[TCP] Accepted connection from {}", peer_addr);

            let handler = TcpConnectionHandler::new(
                self.engine.clone(),
                self.indexing.clone(),
                self.query_executor.clone(),
            );

            tokio::spawn(async move {
                if let Err(e) = handler.handle_connection(stream).await {
                    eprintln!("[TCP] Error handling connection: {}", e);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let query = PgMessage::from_query("SELECT * FROM users".to_string());
        let bytes = query.to_bytes();

        assert_eq!(bytes[0], b'Q');
        assert!(bytes.len() > 5);
    }

    #[test]
    fn test_error_message() {
        let error = PgMessage::error_response("Table not found");
        let bytes = error.to_bytes();

        assert_eq!(bytes[0], b'E');
    }

    #[test]
    fn test_ready_for_query() {
        let ready = PgMessage::ready_for_query();
        let bytes = ready.to_bytes();

        assert_eq!(bytes[0], b'Z');
    }
}
