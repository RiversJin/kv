use bytes::Bytes;
use itertools::Itertools;
use itertools::PeekingNext;
use tokio::sync::RwLock;
use crate::error::*;
use core::time;
use std::sync::LazyLock;
use std::sync::Arc;
use crate::parser::OK_RESP;
use crate::{context::Context, parser::{RespRequest, RespValue}};
use crate::command_table::{RouteHandler, ROUTE_MAP};

static STRING_MAP: LazyLock<RwLock<std::collections::HashMap<String, String>>> = LazyLock::new(|| {
    RwLock::new(std::collections::HashMap::new())
});

#[derive(Debug)]
enum ExistCond {
    None,
    // if key not exists
    NX,
    // if key exists
    XX,
}

#[derive(Debug)]
enum Expiration {
    None,
    KeepTTL,
    Deadline(chrono::DateTime<chrono::Utc>),
}

#[derive(Debug)]
struct SetOption {
    exist_cond: Option<ExistCond>,
    // return old value if key exists
    get: bool,
    expire: Expiration,
}

struct SetCommand {
    key: Bytes,
    value: Bytes,
    option: SetOption,
}

fn prase_set_command(request: RespRequest) -> Result<SetCommand> {
    let mut arg_iter = request.args.into_iter();
    let wrong_arg_number = || Error::WrongArgNumber("set".into());

    let key = arg_iter.next().ok_or_else(wrong_arg_number)?.as_bytes()?.clone();
    let value = arg_iter.next().ok_or_else(wrong_arg_number)?.as_bytes()?.clone();

    let mut option = SetOption {
        exist_cond: None,
        get: false,
        expire: Expiration::None,
    };

    let mut iter = arg_iter.peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str()? {
            "EX" => {
                let expire_sec = iter.next().ok_or(Error::Syntax)?.as_i64()?;
                let deadline = chrono::Utc::now() + chrono::Duration::seconds(expire_sec);
                option.expire = Expiration::Deadline(deadline);
            }
            "PX" => {
                let expire_ms = iter.next().ok_or(Error::Syntax)?.as_i64()?;
                let deadline = chrono::Utc::now() + chrono::Duration::milliseconds(expire_ms);
                option.expire = Expiration::Deadline(deadline);
            }
            "NX" => {
                option.exist_cond = Some(ExistCond::NX);
            }
            "XX" => {
                option.exist_cond = Some(ExistCond::XX);
            }
            "KEEPTTL" => {
                option.expire = Expiration::KeepTTL;
            }
            "GET" => {
                option.get = true;
            }
            _ => Err(Error::Syntax)?
        }
    }

    Ok(SetCommand{
        key,
        value,
        option,
    })
}

async fn set(_context : Arc<Context>, request: RespRequest) -> Result<RespValue> {
    todo!("set")
}

async fn get(_context : Arc<Context>, request: RespRequest) -> Result<RespValue> {
    todo!("get")
}