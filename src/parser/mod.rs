use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use bytes::{Bytes, BytesMut};
use std::error::Error;
use async_recursion::async_recursion;

#[derive(Debug, Clone)]
pub enum RespValue {
    SimpleString(Bytes),
    Error(Bytes),
    Integer(i64),
    BulkString(Option<Bytes>),
    Array(Vec<RespValue>),
}

#[derive(Debug)]
pub struct RespRequest {
    pub command: Bytes,
    pub arguments: Vec<RespValue>,
}

pub struct RespParser<R: AsyncReadExt> {
    reader: BufReader<R>,
}

impl<R: AsyncReadExt + Unpin + Send> RespParser<R> {
    pub fn new(reader: R) -> Self{
        RespParser {
            reader: BufReader::new(reader),
        }
    }

    #[async_recursion]
    pub async fn parse(&mut self) -> Result<RespValue, Box<dyn Error>> {
        let mut line = Vec::new();
        self.reader.read_until(b'\n', &mut line).await?;

        let length = line.len();
        if length < 3 {
            return Err("Invalid line".into());
        }

        let first_char = line[0];
        
        match first_char {
            // simple string
            b'+' => {
                let trimed = Bytes::from(line).slice(1..length - 2);
                Ok(RespValue::SimpleString(trimed))
            }

            // error
            b'-' => {
                let trimed = Bytes::from(line).slice(1..length - 2);
                Ok(RespValue::Error(trimed))
            }

            // integer
            b':' => {
                let value: u64 = std::str::from_utf8(&line[1..length - 2])?.parse()?;
                Ok(RespValue::Integer(value as i64))
            }

            // bulk string
            b'$' => {
                let cnt = std::str::from_utf8(&line[1..length - 2])?.parse::<i64>()?;
                if cnt == -1 {
                    return Ok(RespValue::BulkString(None));
                }

                if cnt == 0 {
                    return Ok(RespValue::BulkString(Some(Bytes::new())));
                }

                let mut buf = BytesMut::with_capacity(cnt as usize);
                self.reader.read_exact(&mut buf).await?;
                self.reader.read_until(b'\n', &mut line).await?;
                Ok(RespValue::BulkString(Some(buf.freeze())))
            }

            // array
            b'*' => {
                let cnt: usize = std::str::from_utf8(&line[1..length - 2])?.parse()?;
                let mut array = Vec::with_capacity(cnt);

                for _ in 0..cnt {
                    let value = self.parse().await?;
                    array.push(value);
                }
                Ok(RespValue::Array(array))
            }
            _ => Err(format!("Invalid character: {}", first_char).into())
        }
    }

    pub async fn parse_request(&mut self) -> Result<RespRequest, Box<dyn Error>> {
        match self.parse().await? {
            RespValue::Array(values) => {
                let command = values
                    .get(0)
                    .and_then(|v| match v {
                        // resp command must be a bulk string
                        RespValue::BulkString(Some(cmd)) => Some(cmd.clone()),
                        _ => None,
                    })
                    .ok_or(format!("Invalid command <{:?}>", values))?;

                let args = values.iter().skip(1).cloned().collect();
                Ok(RespRequest {
                    command,
                    arguments: args,
                })
            }

            content => Err(format!("Invalid request <{:?}>", content).into()),
        }
    }

}