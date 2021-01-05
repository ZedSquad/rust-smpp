use futures::prelude::*;
use log::*;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::task;

type Result<T> =
    std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() {
    env_logger::init();

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(app()) {
        Ok(_) => info!("Done"),
        Err(e) => error!("Error launching: {}", e),
    };
}

async fn app() -> Result<()> {
    info!("Starting");
    let host_port = "0.0.0.0:8080";
    let mut listener = TcpListener::bind(host_port).await?;
    info!("Bound on {}", host_port);
    loop {
        let (socket, addr) = listener.accept().await?;
        tokio::spawn(async move {
            debug!("Connection {} - opened", addr);
            let result = process(socket).await;
            log_result(result, addr);
        });
    }
}

fn log_result(closed_due_to_exit: Result<bool>, addr: SocketAddr) {
    match closed_due_to_exit {
        Ok(true) => {
            debug!("Connection {} - closed by us due to 'exit' received", addr)
        }
        Ok(false) => debug!(
            "Connection {} - closed since client closed the socket",
            addr
        ),
        Err(e) => {
            error!("Connection {} - closed due to error: {}", addr, e)
        }
    }
}

async fn process(mut socket: TcpStream) -> Result<bool> {
    let mut buf = [0; 1024];
    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            return Ok(false); // Socket closed
        }

        let cmd = String::from_utf8_lossy(&buf[0..n]);
        let response = if cmd.trim() == "exit" {
            return Ok(true);
        } else {
            "Send 'exit' to close the connection.\n"
        };

        socket.write_all(response.as_bytes()).await?
    }
}
