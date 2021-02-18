use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use smpp::async_result::AsyncResult;
use smpp::smsc;
use smpp::smsc::{BindData, BindError, SmscConfig, SmscLogic};

const TEST_BIND_URL: &str = "127.0.0.1";

static PORT: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(8080));

/// Setup for running tests that send and receive PDUs
pub struct TestSetup {
    server: TestServer,
}

#[allow(dead_code)]
impl TestSetup {
    pub fn new() -> Self {
        let server = TestServer::start().unwrap();
        Self { server }
    }

    pub fn new_with_logic<L: SmscLogic + Send + Sync + 'static>(
        smsc_logic: L,
    ) -> Self {
        let server = TestServer::start_with_logic(smsc_logic).unwrap();
        Self { server }
    }

    async fn send_exp(
        &self,
        input: &[u8],
        expected_output: &[u8],
    ) -> TestClient {
        let mut client = TestClient::connect_to(&self.server).await.unwrap();
        client.stream.write(input).await.unwrap();

        let resp = client.read_n(expected_output.len()).await;
        assert_eq!(bytes_as_string(&resp), bytes_as_string(expected_output));
        client
    }

    pub fn send_and_expect_response(
        &self,
        input: &[u8],
        expected_output: &[u8],
    ) {
        self.server.runtime.block_on(async {
            self.send_exp(input, expected_output).await;
        })
    }

    pub fn send_and_expect_error_response(
        &self,
        input: &[u8],
        expected_output: &[u8],
        expected_error: &str,
    ) {
        self.server.runtime.block_on(async {
            let mut client = self.send_exp(input, expected_output).await;

            // Since this is an error, server should drop the connection
            let resp = client.stream.read_u8().await.unwrap_err();
            assert_eq!(&resp.to_string(), expected_error);
        })
    }
}

// TODO: allow and disallow binding via username+password (pluggable validator)
// TODO: receive MT (pluggable handler)
// TODO: return DR
// TODO: return MO
// Later: client app + system test that allows us to compare with CloudHopper
// Later: smpp session states (spec 2.2)
// Later: sc_interface_version TLV in bind response
// Later: Check interface versions in binds and responses, and submit_sm
// Later: all PDU types and formats
fn next_port() -> usize {
    return PORT.fetch_add(1, Ordering::Relaxed);
}

/// A test server listening on the test port
pub struct TestServer {
    pub runtime: Runtime,
    pub bind_address: String,
}

#[allow(dead_code)]
impl TestServer {
    pub fn start() -> AsyncResult<TestServer> {
        struct Logic {}

        #[async_trait]
        impl SmscLogic for Logic {
            async fn bind(
                &self,
                _bind_data: &BindData,
            ) -> Result<(), BindError> {
                Ok(())
            }
        }

        Self::start_with_logic(Logic {})
    }

    pub fn start_with_logic<L: SmscLogic + Send + Sync + 'static>(
        smsc_logic: L,
    ) -> AsyncResult<TestServer> {
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
        server
            .runtime
            .spawn(smsc::app(smsc_config, Arc::new(Mutex::new(smsc_logic))));

        Ok(server)
    }
}

/// A client that is able to connect to the server
pub struct TestClient {
    pub stream: TcpStream,
}

#[allow(dead_code)]
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

    pub async fn write_str(&mut self, output: &str) -> AsyncResult<()> {
        self.stream.write_all(output.as_bytes()).await?;
        Ok(())
    }

    pub async fn read_string(&mut self) -> AsyncResult<String> {
        let mut buf = vec![0; 1024];
        let n = self.stream.read(&mut buf).await?;
        let ret = String::from_utf8_lossy(&buf[..n]).to_string();
        Ok(ret)
    }

    pub async fn read_n_maybe(
        &mut self,
        n: usize,
    ) -> Result<Vec<u8>, io::Error> {
        let mut bytes: Vec<u8> = Vec::with_capacity(n);

        while bytes.len() < n {
            bytes.push(self.stream.read_u8().await?);
        }
        Ok(bytes)
    }

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
