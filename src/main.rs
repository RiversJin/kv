use std::sync::LazyLock;
use anyhow::Result;
use clap::Parser;
use kv::{connection::Connection, utils};

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long, default_value = "9090")]
    port: u16,
}

static ARG: LazyLock<Args> = LazyLock::new(|| Args::parse());

#[tokio::main]
async fn main() -> Result<()> {
    let port = ARG.port;
    println!("Port: {}", port);

    let listener = utils::bind_port(port).await?;
    println!("Listening on: {}", listener.local_addr()?);

    loop {
        tokio::select! {
            Ok((socket, addr)) = listener.accept() => {
                println!("Accepted connection from: {}", addr);
                tokio::spawn(async move {
                    let mut conn = Connection::new(socket);
                    conn.serve_loop().await;
                });
            }

            _ = tokio::signal::ctrl_c() => {
                println!("Shutting down");
                break;
            }
        }
    }
    Ok(())
}