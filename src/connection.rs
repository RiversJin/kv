use core::str;
use std::{sync::Arc, time::Duration};
use bytes::Bytes;
use tokio::net::TcpStream;
use crate::{context::Context, parser::{RespParser, RespRequest, RespValue}};
use anyhow::Result;


pub struct Connection {
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

    pub async fn read_request(&mut self) -> Result<RespRequest> {
        let mut parser = RespParser::new(&mut self.reader);
        parser.parse_request().await
    }

    pub async fn write_response(&mut self, response: RespValue) -> Result<()> {
        response.write(&mut self.writer).await
    }

    async fn process(&mut self, context: Arc<Context>, req: RespRequest) -> Result<RespValue> {
        println!("Processing request: {:?}", req);
        let command = str::from_utf8(req.command.as_ref())?;
        let handler = crate::command_table::get_handler(command)?;
        tokio::time::timeout(context.timeout.unwrap_or(Duration::from_secs(5)), handler(context, req)).await?
    }

    pub async fn serve_loop(&mut self) {
        loop {
            let req = match self.read_request().await {
                Ok(req) => req,
                Err(e) => {
                    eprintln!("Error reading request: {}", e);
                    return;
                }
            };

            let context = Arc::new(Context::new(Some(Duration::from_secs(5)), 3));
            let result = self.process(context, req).await;

            let response = match result {
                Ok(response) => response,
                Err(e) => RespValue::Error(Bytes::from(e.to_string()))
            };

            if let Err(e) = self.write_response(response).await {
                eprintln!("Error writing response: {}", e);
                return;
            }
        }
    }
}
