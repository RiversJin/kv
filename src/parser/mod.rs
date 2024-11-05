use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use bytes::{Bytes, BytesMut};
use std::error::Error;
use async_recursion::async_recursion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(Bytes),
    Error(Bytes),
    Integer(i64),
    BulkString(Option<Bytes>),
    Array(Vec<RespValue>),
}

pub trait RespWriter: tokio::io::AsyncWrite + Unpin + Send {}
impl<T> RespWriter for T where T: tokio::io::AsyncWrite + Unpin + Send {}
    

impl RespValue {
    fn get_expected_len(&self) -> usize {
        match self {
            RespValue::SimpleString(s) | RespValue::Error(s) => s.len() + 3, // +3 for +\r\n or -\r\n
            RespValue::Integer(_) => 32, // 32 is enough for i64
            RespValue::BulkString(s) => s.as_ref().map(|s| s.len()).unwrap_or(0) + 16, // for length or -1 + $\r\n and etc
            RespValue::Array(arr) => {
                let mut len = 3; // for *\r\n
                for v in arr {
                    len += v.get_expected_len();
                }
                len
            }
        }
    }

    async fn write_simple_string(s: &Bytes, writer: &mut impl RespWriter) -> Result<(), Box<dyn Error>> {
        writer.write_all(b"+").await?;
        writer.write_all(&s).await?;
        writer.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_error(s: &Bytes, writer: &mut impl RespWriter) -> Result<(), Box<dyn Error>> {
        writer.write_all(b"-").await?;
        writer.write_all(&s).await?;
        writer.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_integer(i: i64, writer: &mut impl RespWriter) -> Result<(), Box<dyn Error>> {
        writer.write_all(b":").await?;
        writer.write_all(i.to_string().as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_bulk_string(s: &Option<Bytes>, writer: &mut impl RespWriter) -> Result<(), Box<dyn Error>> {
        match s {
            Some(s) => {
                writer.write_all(b"$").await?;
                writer.write_all(s.len().to_string().as_bytes()).await?;
                writer.write_all(b"\r\n").await?;
                writer.write_all(&s).await?;
                writer.write_all(b"\r\n").await?;
            }
            None => {
                writer.write_all(b"$-1\r\n").await?;
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn write_array(arr: &[RespValue], writer: &mut impl RespWriter) -> Result<(), Box<dyn Error>> {
        writer.write_all(b"*").await?;
        writer.write_all(arr.len().to_string().as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
        for v in arr {
            v.write(writer).await?;
        }
        Ok(())
    }

    pub async fn write(&self, writer: &mut impl RespWriter) -> Result<(), Box<dyn Error>>{
        let expected_len = self.get_expected_len();
        let mut writer = BufWriter::with_capacity(expected_len, writer);

        match self {
            RespValue::SimpleString(value) => Self::write_simple_string(value, &mut writer).await?,
            RespValue::Error(value) => Self::write_error(value, &mut writer).await?,
            RespValue::Integer(value) => Self::write_integer(*value, &mut writer).await?,
            RespValue::BulkString(value) => Self::write_bulk_string(value, &mut writer).await?,
            RespValue::Array(value) => Self::write_array(value, &mut writer).await?,
        }

        writer.flush().await?;
        Ok(())
    }
}


#[derive(Debug)]
pub struct RespRequest {
    pub command: Bytes,
    pub args: Vec<RespValue>,
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
                    args,
                })
            }

            content => Err(format!("Invalid request <{:?}>", content).into()),
        }
    }

}