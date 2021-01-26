use bytes::{Buf, BytesMut};
use log::*;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, TryAcquireError};

use crate::pdu::pdu_types::{COctetString, Integer4};
use crate::pdu::{
    BindTransmitterRespPdu, CheckOutcome, Pdu, MAX_LENGTH_SYSTEM_ID,
};
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
        let pdu = connection.read_pdu().await?;
        if let Some(pdu) = pdu {
            info!("<= {:?}", pdu);
            let response = Pdu::BindTransmitterResp(BindTransmitterRespPdu {
                sequence_number: Integer4::new(0x01),
                system_id: COctetString::new(
                    &config.system_id,
                    MAX_LENGTH_SYSTEM_ID,
                ),
            });
            connection.write_pdu(&response).await?;
        } else {
            return Ok(false);
        }
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
            Ok(CheckOutcome::Ok) => {
                // Pdu::check moved us to the end, so position is length
                let len = buf.position() as usize;

                // Rewind and parse
                buf.set_position(0);
                let pdu = Pdu::parse(&mut buf)?;

                // Parsing succeeded, so consume the bytes from buffer and return
                self.buffer.advance(len);
                Ok(Some(pdu))
            }
            Ok(CheckOutcome::Incomplete) => Ok(None), // Try again when we have more
            Err(e) => Err(e),                         // Failed (e.g. too long)
        }
    }

    async fn write_pdu(&mut self, pdu: &Pdu) -> Result<()> {
        pdu.write(&mut self.tcp_stream).await
    }
}
