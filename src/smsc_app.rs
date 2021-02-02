use bytes::{Buf, BytesMut};
use core::fmt::{Display, Formatter};
use log::*;
use std::error;
use std::io;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, TryAcquireError};

use crate::async_result::AsyncResult;
use crate::pdu::{
    BindTransmitterRespPdu, CheckOutcome, GenericNackPdu, Pdu, PduParseError,
    PduParseErrorKind,
};
use crate::smsc_config::SmscConfig;

const ERROR_STATUS_FAILED_TO_PARSE_OTHER_PDU: u32 = 0x00010001;
const ERROR_STATUS_PDU_HEADER_INVALID: u32 = 0x00010002;
const ERROR_STATUS_UNEXPECTED_PDU_TYPE: u32 = 0x00010003;

pub fn run(config: SmscConfig) -> AsyncResult<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app(config))
}

pub async fn app(config: SmscConfig) -> AsyncResult<()> {
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

fn log_result(closed_by_us: Result<bool, ProcessError>, addr: SocketAddr) {
    match closed_by_us {
        Ok(true) => {
            info!("Connection {} - closed by us", addr)
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

#[derive(Debug)]
struct UnexpectedPduType {
    /*    command_id: i32,
sequence_number: i32,*/}

#[derive(Debug)]
enum ProcessError {
    PduParseError(PduParseError),
    UnexpectedPduType(UnexpectedPduType),
    IoError(io::Error),
}

impl ProcessError {
    fn new_unexpected_pdu_type() -> Self {
        ProcessError::UnexpectedPduType(UnexpectedPduType {})
    }
}

impl From<PduParseError> for ProcessError {
    fn from(pdu_parse_error: PduParseError) -> Self {
        ProcessError::PduParseError(pdu_parse_error)
    }
}

impl From<io::Error> for ProcessError {
    fn from(io_error: io::Error) -> Self {
        ProcessError::IoError(io_error)
    }
}

impl Display for ProcessError {
    fn fmt(
        &self,
        formatter: &mut Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        let s = match self {
            ProcessError::PduParseError(e) => e.to_string(),
            // Issue#1: UnexpectedPduType should have command_id
            // and sequence_number
            ProcessError::UnexpectedPduType(_) => {
                format!("Unexpected PDU type")
            }
            ProcessError::IoError(e) => e.to_string(),
        };
        formatter.write_str(&s)
    }
}

impl error::Error for ProcessError {}

async fn process(
    tcp_stream: TcpStream,
    config: &SmscConfig,
) -> Result<bool, ProcessError> {
    let mut connection = SmppConnection::new(tcp_stream);
    loop {
        let pdu = connection.read_pdu().await;
        match pdu {
            Ok(pdu) => {
                if let Some(pdu) = pdu {
                    match handle_pdu(pdu, config).await {
                        Ok(response) => connection.write_pdu(&response).await?,
                        Err(e) => {
                            // Couldn't handle this PDU type.  Send a nack...
                            connection
                                .write_pdu(&Pdu::GenericNack(
                                    GenericNackPdu::new(
                                        ERROR_STATUS_UNEXPECTED_PDU_TYPE,
                                        0, // Issue#1: all pdus should have a
                                           // sequence_number, and we should
                                           // use it here: pdu.sequence_number
                                    ),
                                ))
                                .await?;
                            // ...and Drop the connection.
                            return Err(e);
                        }
                    }
                } else {
                    // Client closed the connection
                    return Ok(false);
                }
            }
            Err(pdu_parse_error) => {
                // Respond with an error
                let response = handle_pdu_parse_error(&pdu_parse_error);
                connection.write_pdu(&response).await?;

                // Then return the error, so we drop the connection
                return Err(pdu_parse_error.into());
            }
        }
    }
}

fn handle_pdu_parse_error(error: &PduParseError) -> Pdu {
    match error.command_id {
        Some(0x00000002) => {
            // Parsing failed, so we don't know the sequence number
            Pdu::BindTransmitterResp(BindTransmitterRespPdu::new_failure(0))
        }
        // For any PDU type we're not set up for, send generic_nack
        Some(_) => Pdu::GenericNack(GenericNackPdu::new(
            ERROR_STATUS_FAILED_TO_PARSE_OTHER_PDU,
            0,
        )),
        // If we don't even know the PDU type, send generic_nack
        None => Pdu::GenericNack(GenericNackPdu::new(
            ERROR_STATUS_PDU_HEADER_INVALID,
            0,
        )),
    }
}

async fn handle_pdu(
    pdu: Pdu,
    config: &SmscConfig,
) -> Result<Pdu, ProcessError> {
    info!("<= {:?}", pdu);
    match pdu {
        Pdu::BindTransmitter(pdu) => {
            Ok(Pdu::BindTransmitterResp(BindTransmitterRespPdu::new(
                pdu.sequence_number.value,
                &config.system_id,
            )))
        }
        // Issue#1: all pdus should have a command id: Some(pdu.command_id)
        _ => Err(ProcessError::new_unexpected_pdu_type()),
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

    async fn read_pdu(&mut self) -> Result<Option<Pdu>, PduParseError> {
        loop {
            if let Some(pdu) = self.parse_pdu()? {
                return Ok(Some(pdu));
            }

            if 0 == self.tcp_stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(PduParseError::new(
                        PduParseErrorKind::OtherIoError,
                        "Connection closed by peer.",
                        None,
                        None,
                    ));
                }
            }
        }
    }

    fn parse_pdu(&mut self) -> Result<Option<Pdu>, PduParseError> {
        let mut buf = Cursor::new(&self.buffer[..]);
        match Pdu::check(&mut buf) {
            Ok(CheckOutcome::Ready) => {
                // Pdu::check moved us to the end, so position is length
                let len = buf.position() as usize;

                // Rewind and parse
                buf.set_position(0);
                let pdu = Pdu::parse(&mut buf)?;

                // Parsing succeeded, so consume the bytes from buffer and return
                self.buffer.advance(len);
                Ok(Some(pdu))
            }
            // Try again when we have more
            Ok(CheckOutcome::Incomplete) => Ok(None),
            // Failed (e.g. too long)
            Err(e) => Err(PduParseError::new(
                PduParseErrorKind::OtherIoError,
                &e.message,
                None,
                e.io_errorkind,
            )),
            // Issue#1: it would be good to respond with a specific error here,
            // instead of generic_nack.  That should be possible in some cases
            // if we can read the PDU header before we reject it.  It's not
            // too bad to do this though, because the PDU is actually
            // malformed, so not knowing what type it is is forgivable.
        }
    }

    async fn write_pdu(&mut self, pdu: &Pdu) -> io::Result<()> {
        pdu.write(&mut self.tcp_stream).await
    }
}
