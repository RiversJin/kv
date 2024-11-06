use std::sync::Arc;
use anyhow::Result;
use crate::{context::Context, parser::{RespRequest, RespValue}, utils::get_built_info};
mod string;
use crate::command_table::{RouteHandler, ROUTE_MAP};

#[router_macro::route("PING")]
async fn ping(_context : Arc<Context>, _request: RespRequest) -> Result<RespValue> {
    Ok(RespValue::SimpleString("PONG".into()))
}

#[router_macro::route("VERSION")]
async fn version(_context : Arc<Context>, _request: RespRequest) -> Result<RespValue> {
    let version_info = get_built_info();
    Ok(RespValue::BulkString(Some(version_info.into())))
}