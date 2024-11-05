use bytes::Bytes;
use tokio::sync::RwLock;
use core::str;
use std::sync::LazyLock;
use std::{error::Error, sync::Arc};
use crate::parser::OK_RESP;
use crate::{context::Context, parser::{RespRequest, RespValue}};
use crate::command_table::{RouteHandler, ROUTE_MAP};

static STRING_MAP: LazyLock<RwLock<std::collections::HashMap<String, String>>> = LazyLock::new(|| {
    RwLock::new(std::collections::HashMap::new())
});

#[router_macro::route("SET")]
async fn set(_context : Arc<Context>, request: RespRequest) -> Result<RespValue, Box<dyn Error>> {
    let (key, value) =  match request.args.as_slice() {
        [RespValue::SimpleString(key), RespValue::SimpleString(value)] => unsafe{ (str::from_utf8_unchecked(key), str::from_utf8_unchecked(value)) },
        _ => return Err("SET command must have 2 arguments".into())
    };

    let mut map = STRING_MAP.write().await;
    map.insert(key.to_string(), value.to_string());

    Ok(OK_RESP.clone())
}

#[router_macro::route("GET")]
async fn get(_context : Arc<Context>, request: RespRequest) -> Result<RespValue, Box<dyn Error>> {
    let key =  match request.args.as_slice() {
        [RespValue::SimpleString(key)] => unsafe{ str::from_utf8_unchecked(key) },
        _ => return Err("GET command must have 1 argument".into())
    };

    let map = STRING_MAP.read().await;
    let value = map.get(key);

    let value = value.map(|v| Bytes::copy_from_slice(v.as_bytes()));
    
    Ok(RespValue::BulkString(value))
}