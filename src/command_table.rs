use std::{error::Error, future::Future, pin::Pin, sync::Arc};
use crate::{context::Context, parser::{RespRequest, RespValue}};

pub type RouteHandler = fn(context: Arc<Context>, request: RespRequest) -> Pin<Box<dyn Future<Output = Result<RespValue, Box<dyn Error>>> + Send>>;
router_macro::init_route_map!(ROUTER);

pub fn get_handler(command: &str) -> Result<&'static RouteHandler, Box<dyn Error>> {
    ROUTER.get(command).ok_or(format!("Command <{}> not found", command).into())
}


// test
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn is_ping_exsits() -> Result<(), Box<dyn Error>> {
        let command = "PING";
        let handler = ROUTER.get(command).unwrap();
        let context = Arc::new(Context::new(None, 3));

        let request = RespRequest{
            command: command.into(),
            args: vec![],
        };

        let response = handler(context, request).await?;

        assert_eq!(response, RespValue::SimpleString("PONG".into()));
        Ok(())
    }
}