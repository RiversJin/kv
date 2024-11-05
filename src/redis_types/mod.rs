use std::{error::Error, sync::Arc};
use crate::{context::Context, parser::{RespRequest, RespValue}};
mod string;
use crate::command_table::{RouteHandler, ROUTE_MAP};

#[router_macro::route("PING")]
async fn ping(_context : Arc<Context>, _request: RespRequest) -> Result<RespValue, Box<dyn Error>> {
    Ok(RespValue::SimpleString("PONG".into()))
}