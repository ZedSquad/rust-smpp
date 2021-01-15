use bytes::{Buf, BytesMut};
use log::*;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, TryAcquireError};

use crate::result::Result;
use crate::smsc_config::SmscConfig;

pub fn run(config: SmscConfig) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app(config))
}

pub async fn app(config: SmscConfig) -> Result<()> {
    info!("Starting");
    let sem = Arc::new(Semaphore::new(config.max_open_sockets));
    let listener = TcpListener::bind(&config.bind_address).await?;
    info!("Bound on {}", config.bind_address);

    loop {
        let (tcp_stream, socket_addr) = listener.accept().await?;
        let sem_clone = Arc::clone(&sem);
        let config_clone = config.clone();
        tokio::spawn(async move {
            let aqu = sem_clone.try_acquire();
            match aqu {
                Ok(_guard) => {
                    info!("Connection {} - opened", socket_addr);
                    let result = process(tcp_stream, &config_clone).await;
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

async fn process(tcp_stream: TcpStream, config: &SmscConfig) -> Result<bool> {
    let mut connection = SmppConnection::new(tcp_stream);
    loop {
        info!("<= {:?}", connection.read_pdu().await?.unwrap());
        let response = Pdu::BindTransmitterResp(BindTransmitterRespPdu {
            sequence_number: 0x01,
            system_id: String::from(&config.system_id),
        });
        connection.write_pdu(&response).await?;
    }
    //Ok(false) false means client closed, true means we closed
}

#[derive(Debug)]
enum Pdu {
    BindTransmitter(BindTransmitterPdu),
    BindTransmitterResp(BindTransmitterRespPdu),
}

enum CheckResult {
    Ok,
    Incomplete,
    Error(Box<dyn std::error::Error + Send + Sync>),
}

impl Pdu {
    fn parse(bytes: &mut Cursor<&[u8]>) -> Result<Pdu> {
        //let length = self.tcp_stream.read_u32().await?;

        Ok(Pdu::BindTransmitter(BindTransmitterPdu {
            sequence_number: 0x12,
            system_id: String::from(""),
            password: String::from(""),
            system_type: String::from(""),
            interface_version: 0x99,
            addr_ton: 0x99,
            addr_npi: 0x99,
            address_range: String::from("rng"),
        }))
    }

    fn check(bytes: &mut Cursor<&[u8]>) -> CheckResult {
        CheckResult::Ok
    }
}

#[derive(Debug)]
struct BindTransmitterPdu {
    sequence_number: u32,
    system_id: String,
    password: String,
    system_type: String,
    interface_version: u8,
    addr_ton: u8,
    addr_npi: u8,
    address_range: String,
}

impl BindTransmitterPdu {
    async fn write(&self, _tcp_stream: &mut TcpStream) -> Result<()> {
        todo!()
    }
}

#[derive(Debug)]
struct BindTransmitterRespPdu {
    sequence_number: u32,
    system_id: String,
}

impl BindTransmitterRespPdu {
    async fn write(&self, tcp_stream: &mut TcpStream) -> Result<()> {
        // TODO: check max length
        tcp_stream
            .write_u32((16 + self.system_id.len() + 1) as u32)
            .await?; // length
        tcp_stream.write_u32(0x80000002).await?; // command_id: bind_transmitter_resp
        tcp_stream.write_u32(0).await?; // command_status
        tcp_stream.write_u32(self.sequence_number).await?;
        // TODO: check allowed characters (on creation and/or here)
        tcp_stream.write_all(self.system_id.as_bytes()).await?;
        tcp_stream.write_u8(0x00).await?;
        Ok(())
    }
}

struct SmppConnection {
    tcp_stream: TcpStream,
    buffer: BytesMut,
}

impl SmppConnection {
    pub fn new(tcp_stream: TcpStream) -> SmppConnection {
        SmppConnection {
            tcp_stream,
            buffer: BytesMut::with_capacity(4096),
        }
    }

    async fn read_pdu(&mut self) -> Result<Option<Pdu>> {
        loop {
            if let Some(pdu) = self.parse_pdu()? {
                return Ok(Some(pdu));
            }

            if 0 == self.tcp_stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    fn parse_pdu(&mut self) -> Result<Option<Pdu>> {
        let mut buf = Cursor::new(&self.buffer[..]);
        match Pdu::check(&mut buf) {
            CheckResult::Ok => {
                // Pdu::check moved us to the end, so position is length
                let len = buf.position() as usize;

                // Rewind and parse
                buf.set_position(0);
                let pdu = Pdu::parse(&mut buf)?;

                // Parsing succeeded, so consume the bytes from buffer and return
                self.buffer.advance(len);
                Ok(Some(pdu))
            }
            CheckResult::Incomplete => Ok(None), // Try again when we have more
            CheckResult::Error(e) => Err(e),     // Failed (e.g. too long)
        }
    }

    async fn write_pdu(&mut self, pdu: &Pdu) -> Result<()> {
        match pdu {
            Pdu::BindTransmitter(pdu) => {
                pdu.write(&mut self.tcp_stream).await?
            }
            Pdu::BindTransmitterResp(pdu) => {
                pdu.write(&mut self.tcp_stream).await?
            }
        }
        Ok(())
    }
}
