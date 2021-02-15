use once_cell::sync::Lazy;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use smpp::async_result::AsyncResult;
use smpp::smsc_app;
use smpp::smsc_config::SmscConfig;

const TEST_BIND_URL: &str = "127.0.0.1";

static PORT: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(8080));

#[allow(dead_code)]
pub const BIND_RECEIVER_PDU: &[u8; 0x29] =
    b"\x00\x00\x00\x29\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x02\
    esmeid\0password\0type\0\x34\x00\x00\0";

#[allow(dead_code)]
pub const BIND_RECEIVER_RESP_PDU: &[u8; 0x1b] =
    b"\x00\x00\x00\x1b\x80\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x02\
    TestServer\0";

pub const BIND_TRANSMITTER_PDU: &[u8; 0x29] =
    b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
    esmeid\0password\0type\0\x34\x00\x00\0";

#[allow(dead_code)]
pub const BIND_TRANSMITTER_RESP_PDU: &[u8; 0x1b] =
    b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
    TestServer\0";

#[allow(dead_code)]
pub const BIND_TRANSCEIVER_PDU: &[u8; 0x29] =
    b"\x00\x00\x00\x29\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x02\
    esmeid\0password\0type\0\x34\x00\x00\0";

#[allow(dead_code)]
pub const BIND_TRANSCEIVER_RESP_PDU: &[u8; 0x1b] =
    b"\x00\x00\x00\x1b\x80\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x02\
    TestServer\0";

fn next_port() -> usize {
    return PORT.fetch_add(1, Ordering::Relaxed);
}

/// A test server listening on the test port
pub struct TestServer {
    pub runtime: Runtime,
    pub bind_address: String,
}

impl TestServer {
    pub fn start() -> AsyncResult<TestServer> {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
        let server = TestServer {
            runtime: tokio::runtime::Runtime::new()?,
            bind_address: format!("{}:{}", TEST_BIND_URL, next_port()),
        };

        let smsc_config = SmscConfig {
            bind_address: String::from(&server.bind_address),
            max_open_sockets: 2,
            system_id: String::from("TestServer"),
        };
        server.runtime.spawn(smsc_app::app(smsc_config));

        Ok(server)
    }
}

/// A client that is able to connect to the server
pub struct TestClient {
    pub stream: TcpStream,
}

impl TestClient {
    pub async fn connect_to(server: &TestServer) -> AsyncResult<TestClient> {
        // Force the runtime to actually do something: seems to mean
        // the server is running when we connect to it.  Hopefully
        // there is a better way?
        server
            .runtime
            .spawn(async { sleep(Duration::from_millis(1)).await })
            .await?;

        // Connect to the server, retrying with 10ms delay if we fail
        let mut i: u8 = 0;
        loop {
            match TcpStream::connect(&server.bind_address).await {
                Ok(stream) => return Ok(TestClient { stream }),
                Err(e) => {
                    i += 1;
                    sleep(Duration::from_millis(10)).await;
                    if i > 9 {
                        return Err(e.into());
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub async fn write_str(&mut self, output: &str) -> AsyncResult<()> {
        self.stream.write_all(output.as_bytes()).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn read_string(&mut self) -> AsyncResult<String> {
        let mut buf = vec![0; 1024];
        let n = self.stream.read(&mut buf).await?;
        let ret = String::from_utf8_lossy(&buf[..n]).to_string();
        Ok(ret)
    }

    #[allow(dead_code)]
    pub async fn read_n_maybe(
        &mut self,
        n: usize,
    ) -> Result<Vec<u8>, io::Error> {
        let mut bytes: Vec<u8> = Vec::with_capacity(n);

        while bytes.len() < n {
            bytes.push(self.stream.read_u8().await?)
        }
        Ok(bytes)
    }

    #[allow(dead_code)]
    pub async fn read_n(&mut self, n: usize) -> Vec<u8> {
        self.read_n_maybe(n)
            .await
            .unwrap_or_else(|e| match e.kind() {
                io::ErrorKind::UnexpectedEof => {
                    panic!("Error: Not enough bytes to read.")
                }
                _ => panic!("Error while reading: {}", e),
            })
    }
}

#[allow(dead_code)]
pub fn bytes_as_string(arr: &[u8]) -> String {
    arr.iter()
        .map(|x| format!("{:>02x}", x))
        .collect::<Vec<String>>()
        .join("")
}
