mod built_info;

use anyhow::Result;

pub use built_info::{print_built_info, get_built_info};

pub async fn bind_port(port: u16) -> Result<tokio::net::TcpListener> {
    let addr = format!("[::]:{port}").parse::<std::net::SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    Ok(listener)
}