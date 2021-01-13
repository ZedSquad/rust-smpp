use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use smpp::result::Result;
use smpp::smsc_app;
use smpp::smsc_config::SmscConfig;

const TEST_BIND_URL: &str = "127.0.0.1";

static PORT: Lazy<AtomicUsize> = Lazy::new(|| {
    println!("initializing");
    AtomicUsize::new(8080)
});

fn next_port() -> usize {
    return PORT.fetch_add(1, Ordering::Relaxed);
}

/// A test server listening on the test port
pub struct TestServer {
    pub runtime: Runtime,
    pub bind_address: String,
}

impl TestServer {
    pub fn start() -> Result<TestServer> {
        let server = TestServer {
            runtime: tokio::runtime::Runtime::new()?,
            bind_address: format!("{}:{}", TEST_BIND_URL, next_port()),
        };

        let smsc_config = SmscConfig {
            bind_address: String::from(&server.bind_address),
            max_open_sockets: 2,
        };
        server.runtime.spawn(smsc_app::app(smsc_config));

        Ok(server)
    }
}

/// A client that is able to connect to the server
pub struct TestClient {
    stream: TcpStream,
}

impl TestClient {
    pub async fn connect_to(server: &TestServer) -> Result<TestClient> {
        // Force the runtime to actually do something: seems to mean
        // the server is running when we connect to it.  Hopefully
        // there is a better way?
        server
            .runtime
            .spawn(async { sleep(Duration::from_millis(1)).await })
            .await?;

        // Connect to the server
        Ok(TestClient {
            stream: TcpStream::connect(&server.bind_address).await?,
        })
    }

    pub async fn write_str(&mut self, output: &str) -> Result<()> {
        self.stream.write_all(output.as_bytes()).await?;
        Ok(())
    }

    pub async fn read_string(&mut self) -> Result<String> {
        let mut buf = vec![0; 1024];
        let n = self.stream.read(&mut buf).await?;
        let ret = String::from_utf8_lossy(&buf[..n]).to_string();
        Ok(ret)
    }
}
