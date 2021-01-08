use env_logger::Env;
use log::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, TryAcquireError};

type Result<T> =
    std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct Config {
    bind_address: String,
    max_open_sockets: usize,
}

fn main() {
    let config = Config {
        bind_address: String::from("0.0.0.0:8080"),
        max_open_sockets: 100,
    };

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let res = launch_runtime(config);

    match res {
        Ok(_) => info!("Done"),
        Err(e) => error!("Error launching: {}", e),
    };
}

fn launch_runtime(config: Config) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app(config))
}

async fn app(config: Config) -> Result<()> {
    info!("Starting");
    let sem = Arc::new(Semaphore::new(config.max_open_sockets));
    let listener = TcpListener::bind(&config.bind_address).await?;
    info!("Bound on {}", config.bind_address);

    loop {
        let (tcp_stream, socket_addr) = listener.accept().await?;
        let sem_clone = Arc::clone(&sem);
        tokio::spawn(async move {
            let aqu = sem_clone.try_acquire();
            match aqu {
                Ok(_guard) => {
                    info!("Connection {} - opened", socket_addr);
                    let result = process(tcp_stream).await;
                    log_result(result, socket_addr);
                }
                Err(TryAcquireError::NoPermits) => {
                    error!(
                        "Refused connection {} - too many open sockets",
                        socket_addr
                    );
                }
                Err(TryAcquireError::Closed) => {
                    error!("Unexpected error: semaphore closed");
                }
            }
        });
    }
}

fn log_result(closed_due_to_exit: Result<bool>, addr: SocketAddr) {
    match closed_due_to_exit {
        Ok(true) => {
            info!("Connection {} - closed by us due to 'exit' received", addr)
        }
        Ok(false) => info!(
            "Connection {} - closed since client closed the socket",
            addr
        ),
        Err(e) => {
            error!("Connection {} - closed due to error: {}", addr, e)
        }
    }
}

async fn process(mut socket: TcpStream) -> Result<bool> {
    let mut buf = vec![0; 1024];
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
