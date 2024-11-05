use std::error::Error;
use tokio::net::TcpStream;
use crate::parser::{RespParser, RespRequest, RespValue};


pub struct Connection {
    // stream: TcpStream
    writer: tokio::io::WriteHalf<TcpStream>,
    reader: tokio::io::ReadHalf<TcpStream>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = tokio::io::split(stream);
        Connection {
            writer,
            reader,
        }
    }

    pub async fn read_request(&mut self) -> Result<RespRequest, Box<dyn Error>> {
        let mut parser = RespParser::new(&mut self.reader);
        parser.parse_request().await
    }

    pub async fn write_response(&mut self, response: RespValue) -> Result<(), Box<dyn Error>> {
        response.write(&mut self.writer).await
    }
}

