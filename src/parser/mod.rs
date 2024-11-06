use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use bytes::Bytes;
use std::sync::LazyLock;
use async_recursion::async_recursion;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(Bytes),
    Error(Bytes),
    Integer(i64),
    BulkString(Option<Bytes>),
    Array(Vec<RespValue>),
}

pub static OK_RESP: LazyLock<RespValue> = LazyLock::new(|| RespValue::SimpleString("OK".into()));
pub static NULL_RESP: LazyLock<RespValue> = LazyLock::new(|| RespValue::BulkString(None));

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

    pub fn as_str(&self) -> Result<&str> {
        match self {
            RespValue::SimpleString(s) => Ok(unsafe{ std::str::from_utf8_unchecked(s) }),
            RespValue::BulkString(Some(s)) => Ok(unsafe{ std::str::from_utf8_unchecked(s) }),
            RespValue::BulkString(None) => Err(anyhow!("Null bulk string is invalid")),
            _ => Err(anyhow!("Invalid type to convert to string")),
        }
    }

    async fn write_simple_string(s: &Bytes, writer: &mut impl RespWriter) -> Result<()> {
        writer.write_all(b"+").await?;
        writer.write_all(&s).await?;
        writer.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_error(s: &Bytes, writer: &mut impl RespWriter) -> Result<()> {
        writer.write_all(b"-").await?;
        writer.write_all(&s).await?;
        writer.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_integer(i: i64, writer: &mut impl RespWriter) -> Result<()> {
        writer.write_all(b":").await?;
        writer.write_all(i.to_string().as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_bulk_string(s: &Option<Bytes>, writer: &mut impl RespWriter) -> Result<()> {
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
    async fn write_array(arr: &[RespValue], writer: &mut impl RespWriter) -> Result<()> {
        writer.write_all(b"*").await?;
        writer.write_all(arr.len().to_string().as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
        for v in arr {
            v.write_dispatch(writer).await?;
        }
        Ok(())
    }

    async fn write_dispatch(&self, writer: &mut impl RespWriter) -> Result<()> {
        match self {
            RespValue::SimpleString(value) => Self::write_simple_string(value, writer).await?,
            RespValue::Error(value) => Self::write_error(value, writer).await?,
            RespValue::Integer(value) => Self::write_integer(*value, writer).await?,
            RespValue::BulkString(value) => Self::write_bulk_string(value, writer).await?,
            RespValue::Array(value) => Self::write_array(value, writer).await?,
        }
        Ok(())
    }

    pub async fn write(&self, writer: &mut impl RespWriter) -> Result<()>{
        let expected_len = self.get_expected_len();
        let mut writer = BufWriter::with_capacity(expected_len, writer);
        self.write_dispatch(&mut writer).await?;

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
    pub async fn parse(&mut self) -> Result<RespValue> {
        let mut line = Vec::new();
        self.reader.read_until(b'\n', &mut line).await?;

        let length = line.len();
        if length < 3 {
            if length == 0 {
                return Err(anyhow!("EOF"));
            }

            return Err(anyhow!("Invalid line {:?}", line));
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

                let mut buf = vec![0; cnt as usize + 2]; // +2 for \r\n
                self.reader.read_exact(&mut buf).await?;
                let buf = Bytes::from(buf).slice(0..cnt as usize);
  
                Ok(RespValue::BulkString(Some(buf)))
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
            _ => Err(anyhow!("Invalid character: {}", first_char))
        }
    }

    pub async fn parse_request(&mut self) -> Result<RespRequest> {
        match self.parse().await? {
            RespValue::Array(values) => {
                let command = values
                    .get(0)
                    .and_then(|v| match v {
                        // resp command must be a bulk string
                        RespValue::BulkString(Some(cmd)) => Some(cmd.clone()),
                        _ => None,
                    })
                    .ok_or(anyhow!("Invalid command <{:?}>", values))?;

                let args = values.iter().skip(1).cloned().collect();
                Ok(RespRequest {
                    command,
                    args,
                })
            }

            content => Err(anyhow!("Invalid request <{:?}>", content).into()),
        }
    }

}