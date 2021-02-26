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
use tokio::sync::{Mutex, Semaphore, TryAcquireError};

use crate::async_result::AsyncResult;
use crate::pdu::{
    BindReceiverRespPdu, BindTransceiverRespPdu, BindTransmitterRespPdu,
    CheckOutcome, EnquireLinkRespPdu, GenericNackPdu, Pdu, PduBody,
    PduParseError, PduParseErrorBody, PduStatus, SubmitSmRespPdu,
};
use crate::smsc::{SmscConfig, SmscLogic};

pub fn run<L: SmscLogic + Send + Sync + 'static>(
    config: SmscConfig,
    smsc_logic: L,
) -> AsyncResult<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app(config, Arc::new(Mutex::new(smsc_logic))))
}

struct Smsc {
    connection: Option<Arc<Mutex<SmppConnection>>>,
}

impl Smsc {
    fn add_connection(&mut self, connection: Arc<Mutex<SmppConnection>>) {
        // TODO: stub implementation - will add to some kind of map
        self.connection = Some(connection);
    }
}

pub async fn app<L: SmscLogic + Send + Sync + 'static>(
    config: SmscConfig,
    smsc_logic: Arc<Mutex<L>>,
) -> AsyncResult<()> {
    info!("Starting");
    let sem = Arc::new(Semaphore::new(config.max_open_sockets));
    let listener = TcpListener::bind(&config.bind_address).await?;
    let smsc = Arc::new(Mutex::new(Smsc { connection: None }));
    info!("Bound on {}", config.bind_address);

    loop {
        let (tcp_stream, socket_addr) = listener.accept().await?;
        let connection = SmppConnection::new(tcp_stream, socket_addr);
        let sem_clone = Arc::clone(&sem);
        let config_clone = config.clone();
        let smsc_logic_clone = Arc::clone(&smsc_logic);
        let smsc_clone = Arc::clone(&smsc);
        tokio::spawn(async move {
            let aqu = sem_clone.try_acquire();
            match aqu {
                Ok(_guard) => {
                    info!("Connection {} - opened", connection.socket_addr);
                    let result = process(
                        connection,
                        &config_clone,
                        smsc_logic_clone,
                        smsc_clone,
                    )
                    .await;
                    log_result(result, socket_addr);
                }
                Err(TryAcquireError::NoPermits) => {
                    error!(
                        "Refused connection {} - too many open sockets",
                        connection.socket_addr
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
    command_id: u32,
    sequence_number: u32,
}

#[derive(Debug)]
enum ProcessError {
    PduParseError(PduParseError),
    UnexpectedPduType(UnexpectedPduType),
    IoError(io::Error),
    InternalError(String),
}

impl ProcessError {
    fn new_unexpected_pdu_type(command_id: u32, sequence_number: u32) -> Self {
        ProcessError::UnexpectedPduType(UnexpectedPduType {
            command_id,
            sequence_number,
        })
    }

    fn new_internal_error(message: &str) -> Self {
        ProcessError::InternalError(String::from(message))
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
            ProcessError::UnexpectedPduType(e) => {
                format!(
                    "Unexpected PDU type \
                    (command_id={:#010X}, sequence_number={:#010X})",
                    e.command_id, e.sequence_number
                )
            }
            ProcessError::IoError(e) => e.to_string(),
            ProcessError::InternalError(s) => String::from(s),
        };
        formatter.write_str(&s)
    }
}

impl error::Error for ProcessError {}

async fn process<L: SmscLogic>(
    connection: SmppConnection,
    config: &SmscConfig,
    smsc_logic: Arc<Mutex<L>>,
    smsc: Arc<Mutex<Smsc>>,
) -> Result<bool, ProcessError> {
    struct DisconnectGuard {
        connection: Arc<Mutex<SmppConnection>>,
    }

    impl Drop for DisconnectGuard {
        fn drop(&mut self) {
            let connection = Arc::clone(&self.connection);
            tokio::spawn(async move {
                connection.lock().await.disconnect();
            });
        }
    }

    // Ensure we disconnect connection when we leave this function,
    // even though we are wrapping it in an Arc so it can be accessed
    // from elsewhere.
    let disconnect_guard = DisconnectGuard {
        connection: Arc::new(Mutex::new(connection)),
    };

    process_loop(
        Arc::clone(&disconnect_guard.connection),
        config,
        smsc_logic,
        smsc,
    )
    .await
}

async fn process_loop<L: SmscLogic>(
    connection: Arc<Mutex<SmppConnection>>,
    config: &SmscConfig,
    smsc_logic: Arc<Mutex<L>>,
    smsc: Arc<Mutex<Smsc>>,
) -> Result<bool, ProcessError> {
    loop {
        let pdu = connection.lock().await.read_pdu().await;
        match pdu {
            Ok(pdu) => {
                if let Some(pdu) = pdu {
                    let sequence_number = pdu.sequence_number.value;
                    match handle_pdu(
                        pdu,
                        Arc::clone(&connection),
                        config,
                        Arc::clone(&smsc_logic),
                        Arc::clone(&smsc),
                    )
                    .await
                    {
                        Ok(response) => {
                            info!("=> {:?}", response);
                            connection.lock().await.write_pdu(&response).await?
                        }
                        Err(e) => {
                            // Couldn't handle this PDU type.  Send a nack...
                            connection
                                .lock()
                                .await
                                .write_pdu(
                                    &Pdu::new(
                                        PduStatus::ESME_RINVCMDID as u32,
                                        sequence_number,
                                        GenericNackPdu::new_error().into(),
                                    )
                                    .unwrap(),
                                )
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
                connection.lock().await.write_pdu(&response).await?;

                // Then return the error, so we drop the connection
                return Err(pdu_parse_error.into());
            }
        }
    }
}

fn handle_pdu_parse_error(error: &PduParseError) -> Pdu {
    let sequence_number = error.sequence_number.unwrap_or(1);
    match error.command_id {
        Some(0x00000002) => Pdu::new(
            error.status(),
            sequence_number,
            BindTransmitterRespPdu::new_error().into(),
        )
        .unwrap(),
        // For any PDU type we're not set up for, send generic_nack
        Some(_) => Pdu::new(
            error.status(),
            sequence_number,
            GenericNackPdu::new_error().into(),
        )
        .unwrap(),
        // If we don't even know the PDU type, send generic_nack
        None => Pdu::new(
            error.status(),
            sequence_number,
            GenericNackPdu::new_error().into(),
        )
        .unwrap(),
    }
}

async fn handle_bind_pdu<L: SmscLogic>(
    pdu: Pdu,
    connection: Arc<Mutex<SmppConnection>>,
    config: &SmscConfig,
    smsc_logic: Arc<Mutex<L>>,
    smsc: Arc<Mutex<Smsc>>,
) -> Result<Pdu, ProcessError> {
    let mut command_status = PduStatus::ESME_ROK;

    let ret_body = match pdu.body() {
        PduBody::BindReceiver(body) => {
            let mut logic = smsc_logic.lock().await;
            match logic.bind(body.bind_data()).await {
                Ok(()) => Ok(BindReceiverRespPdu::new(&config.system_id)
                    .unwrap()
                    .into()),
                Err(e) => {
                    command_status = e.into();
                    Ok(BindReceiverRespPdu::new_error().into())
                }
            }
        }
        PduBody::BindTransceiver(body) => {
            let mut logic = smsc_logic.lock().await;
            match logic.bind(body.bind_data()).await {
                Ok(()) => Ok(BindTransceiverRespPdu::new(&config.system_id)
                    .unwrap()
                    .into()),
                Err(e) => {
                    command_status = e.into();
                    Ok(BindTransceiverRespPdu::new_error().into())
                }
            }
        }
        PduBody::BindTransmitter(body) => {
            let mut logic = smsc_logic.lock().await;
            match logic.bind(body.bind_data()).await {
                Ok(()) => Ok(BindTransmitterRespPdu::new(&config.system_id)
                    .unwrap()
                    .into()),
                Err(e) => {
                    command_status = e.into();
                    Ok(BindTransmitterRespPdu::new_error().into())
                }
            }
        }
        // This function should only be called with a Bind PDU
        _ => Err(ProcessError::new_internal_error(
            "handle_bind_pdu called with non-bind PDU!",
        )),
    }?;

    // If we successfully bound, register this connection so we
    // know to use it when we receive deliver_sm PDUs later
    if command_status == PduStatus::ESME_ROK {
        smsc.lock().await.add_connection(connection);
    }

    Pdu::new(command_status as u32, pdu.sequence_number.value, ret_body)
        .map_err(|e| e.into())
}

async fn handle_pdu<L: SmscLogic>(
    pdu: Pdu,
    connection: Arc<Mutex<SmppConnection>>,
    config: &SmscConfig,
    smsc_logic: Arc<Mutex<L>>,
    smsc: Arc<Mutex<Smsc>>,
) -> Result<Pdu, ProcessError> {
    info!("<= {:?}", pdu);
    match pdu.body() {
        PduBody::BindReceiver(_body) => {
            handle_bind_pdu(pdu, connection, config, smsc_logic, smsc)
                .await
                .map_err(|e| e.into())
        }
        PduBody::BindTransmitter(_body) => {
            handle_bind_pdu(pdu, connection, config, smsc_logic, smsc)
                .await
                .map_err(|e| e.into())
        }
        PduBody::BindTransceiver(_body) => {
            handle_bind_pdu(pdu, connection, config, smsc_logic, smsc)
                .await
                .map_err(|e| e.into())
        }

        PduBody::EnquireLink(_body) => Pdu::new(
            PduStatus::ESME_ROK as u32,
            pdu.sequence_number.value,
            EnquireLinkRespPdu::new().into(),
        )
        .map_err(|e| e.into()),

        PduBody::SubmitSm(body) => {
            let mut command_status = PduStatus::ESME_ROK;
            let resp = match smsc_logic.lock().await.submit_sm(body).await {
                Ok(resp) => resp,
                Err(e) => {
                    command_status = e.into();
                    SubmitSmRespPdu::new_error().into()
                }
            };
            Pdu::new(
                command_status as u32,
                pdu.sequence_number.value,
                resp.into(),
            )
            .map_err(|e| e.into())
        }

        _ => Err(ProcessError::new_unexpected_pdu_type(
            pdu.command_id().value,
            pdu.sequence_number.value,
        )),
    }
}

struct SmppConnection {
    tcp_stream: Option<TcpStream>,
    pub socket_addr: SocketAddr,
    buffer: BytesMut,
}

impl SmppConnection {
    pub fn new(
        tcp_stream: TcpStream,
        socket_addr: SocketAddr,
    ) -> SmppConnection {
        SmppConnection {
            tcp_stream: Some(tcp_stream),
            socket_addr,
            buffer: BytesMut::with_capacity(4096),
        }
    }

    async fn read_pdu(&mut self) -> Result<Option<Pdu>, PduParseError> {
        loop {
            if let Some(pdu) = self.parse_pdu()? {
                return Ok(Some(pdu));
            }

            if let Some(tcp_stream) = &mut self.tcp_stream {
                if 0 == tcp_stream.read_buf(&mut self.buffer).await? {
                    if self.buffer.is_empty() {
                        return Ok(None);
                    } else {
                        return Err(PduParseError::new(
                            PduParseErrorBody::NotEnoughBytes,
                        ));
                    }
                }
            } else {
                error!("Attempting to read from a closed connection!");
                return Err(PduParseError::new(
                    PduParseErrorBody::NotEnoughBytes.into(),
                ));
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

                // Parsing succeeded, so consume bytes from buffer and return
                self.buffer.advance(len);
                Ok(Some(pdu))
            }
            // Try again when we have more
            Ok(CheckOutcome::Incomplete) => Ok(None),
            // Failed (e.g. too long)
            Err(e) => Err(e.into()),
            // Issue#1: it would be good to respond with a specific error here,
            // instead of generic_nack.  That should be possible in some cases
            // if we can read the PDU header before we reject it.  It's not
            // too bad to do this though, because the PDU is actually
            // malformed, so not knowing what type it is is forgivable.
        }
    }

    async fn write_pdu(&mut self, pdu: &Pdu) -> io::Result<()> {
        if let Some(tcp_stream) = &mut self.tcp_stream {
            pdu.write(tcp_stream).await
        } else {
            error!("Attempting to write to a closed connection!");
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    fn disconnect(&mut self) {
        self.tcp_stream.take();
    }
}
