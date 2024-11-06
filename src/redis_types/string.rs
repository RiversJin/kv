use bytes::Bytes;
use tokio::sync::RwLock;
use core::str;
use anyhow::{anyhow, Result};
use std::sync::LazyLock;
use std::sync::Arc;
use crate::parser::OK_RESP;
use crate::{context::Context, parser::{RespRequest, RespValue}};
use crate::command_table::{RouteHandler, ROUTE_MAP};

static STRING_MAP: LazyLock<RwLock<std::collections::HashMap<String, String>>> = LazyLock::new(|| {
    RwLock::new(std::collections::HashMap::new())
});

#[router_macro::route("SET")]
async fn set(_context : Arc<Context>, request: RespRequest) -> Result<RespValue> {
    let args = request.args.as_slice();
    if args.len() != 2{
        Err(anyhow!("SET command must have 2 arguments"))?;
    }

    let key = args[0].as_str()?;
    let value = args[1].as_str()?;
    

    let mut map = STRING_MAP.write().await;
    map.insert(key.to_string(), value.to_string());

    Ok(OK_RESP.clone())
}

#[router_macro::route("GET")]
async fn get(_context : Arc<Context>, request: RespRequest) -> Result<RespValue> {
    let args = request.args.as_slice();
    if args.len() != 1{
        Err(anyhow!("GET command must have 1 argument"))?;
    }

    let key = args[0].as_str()?;

    let map = STRING_MAP.read().await;
    let value = map.get(key);

    let value = value.map(|v| Bytes::copy_from_slice(v.as_bytes()));

    Ok(RespValue::BulkString(value))
}