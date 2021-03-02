use bytes::{Buf, BytesMut};
use core::fmt::{Display, Formatter};
use log::*;
use std::io;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{error, time::Duration};
use tokio::io::{split, AsyncReadExt, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, Semaphore, TryAcquireError};
use tokio::time::sleep;

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
    rt.block_on(async move {
        let smsc = Smsc::start(config, smsc_logic).await?;
        loop {
            if let Err(e) = smsc.lock().await.stopped().await {
                return Err(e);
            }
            sleep(Duration::from_millis(100)).await;
            // TODO: notify instead of poll?
        }
    })
}

pub struct Smsc {
    connection: Option<Arc<SmppConnection>>,
}

impl Smsc {
    /// Bind to a TCP socket, and return an object that manages
    /// the list of connected clients.  Spawns a task that deals
    /// with incoming connections, which itself spawns a further
    /// new task each time someone connects.
    pub async fn start<L: SmscLogic + Send + Sync + 'static>(
        smsc_config: SmscConfig,
        smsc_logic: L,
    ) -> AsyncResult<Arc<Mutex<Self>>> {
        info!("Starting SMSC");

        let smsc = Smsc { connection: None };
        let smsc = Arc::new(Mutex::new(smsc));

        let listener = TcpListener::bind(&smsc_config.bind_address).await?;
        info!("Bound on {}", &smsc_config.bind_address);

        // Spawn off a task that deals with incoming connections
        tokio::spawn(listen_loop(
            listener,
            Arc::clone(&smsc),
            smsc_config,
            smsc_logic,
        ));

        Ok(smsc)
    }

    async fn stopped(&self) -> AsyncResult<()> {
        // TODO: check whether we are stopped and return an error if so
        Ok(())
    }

    pub async fn receive_pdu(&mut self, pdu: Pdu) -> AsyncResult<()> {
        // TODO: consider retrying after a delay if unable to match DR
        // TODO: handle MOs separately from DRs
        // TODO: maybe return a deliver_sm_resp on failure?
        match pdu.body() {
            PduBody::DeliverSm(body) => {
                match body.extract_receipted_message_id() {
                    Some(message_id) => {
                        self.receive_pdu_for_message(pdu, message_id).await
                    }
                    None => {
                        Err("Could not extract message ID from supplied PDU."
                            .into())
                    }
                }
            }
            _ => Err("Unexpected PDU type.  Currently we can only \
                    handle deliver_sm PDUs."
                .into()),
        }
    }

    async fn receive_pdu_for_message(
        &mut self,
        pdu: Pdu,
        message_id: String,
    ) -> AsyncResult<()> {
        let conn = self.connection_for_message_id(&message_id).await?;
        // TODO: in order to support a window size to the client, we
        //       will need to put this PDU into a queue rather than writing
        //       it immediately here.
        tokio::spawn(async move {
            conn.write_pdu(&pdu).await.map_err(
                |e| error!("Failed to send PDU to client: {}", e), // TODO: give information about the client here
            )
        });
        Ok(())
    }

    fn add_connection(&mut self, connection: Arc<SmppConnection>) {
        // TODO: stub implementation - will add to some kind of map
        self.connection = Some(connection);
    }

    async fn connection_for_message_id(
        &mut self,
        message_id: &str,
    ) -> AsyncResult<Arc<SmppConnection>> {
        if let Some(connection) = &self.connection {
            Ok(Arc::clone(connection))
        } else {
            Err(format!(
                "No client connection found for message with ID {}",
                message_id
            )
            .into())
        }
    }
}

/// Listen for clients connecting, and spawn a new task every time one does
async fn listen_loop<L: SmscLogic + Send + Sync + 'static>(
    listener: TcpListener,
    smsc: Arc<Mutex<Smsc>>,
    config: SmscConfig,
    logic: L,
) {
    let sem = Arc::new(Semaphore::new(config.max_open_sockets));
    let logic = Arc::new(Mutex::new(logic));
    loop {
        match listener.accept().await {
            Err(e) => {
                error!("Client connection failed: {}", e);
            }
            Ok((tcp_stream, socket_addr)) => {
                tokio::spawn(process_stream(
                    Arc::clone(&sem),
                    SmppConnection::new(tcp_stream, socket_addr),
                    config.clone(),
                    Arc::clone(&logic),
                    Arc::clone(&smsc),
                ));
            }
        }
    }
}

async fn process_stream<L: SmscLogic + Send + Sync + 'static>(
    sem: Arc<Semaphore>,
    connection: SmppConnection,
    config: SmscConfig,
    logic: Arc<Mutex<L>>,
    smsc: Arc<Mutex<Smsc>>,
) {
    let socket_addr = connection.socket_addr.clone();
    let aqu = sem.try_acquire();
    match aqu {
        Ok(_guard) => {
            info!("Connection {} - opened", socket_addr);
            let result = process(connection, config, logic, smsc).await;
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
    config: SmscConfig,
    smsc_logic: Arc<Mutex<L>>,
    smsc: Arc<Mutex<Smsc>>,
) -> Result<bool, ProcessError> {
    struct DisconnectGuard {
        connection: Arc<SmppConnection>,
    }

    impl Drop for DisconnectGuard {
        fn drop(&mut self) {
            futures::executor::block_on(async move {
                self.connection.disconnect().await;
            });
        }
    }

    // Ensure we disconnect connection when we leave this function,
    // even though we are wrapping it in an Arc so it can be accessed
    // from elsewhere.
    let disconnect_guard = DisconnectGuard {
        connection: Arc::new(connection),
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
    connection: Arc<SmppConnection>,
    config: SmscConfig,
    smsc_logic: Arc<Mutex<L>>,
    smsc: Arc<Mutex<Smsc>>,
) -> Result<bool, ProcessError> {
    loop {
        let pdu = connection.read_pdu().await;
        match pdu {
            Ok(pdu) => {
                if let Some(pdu) = pdu {
                    let sequence_number = pdu.sequence_number.value;
                    match handle_pdu(
                        pdu,
                        Arc::clone(&connection),
                        &config,
                        Arc::clone(&smsc_logic),
                        Arc::clone(&smsc),
                    )
                    .await
                    {
                        Ok(response) => {
                            info!("=> {:?}", response);
                            connection.write_pdu(&response).await?
                        }
                        Err(e) => {
                            // Couldn't handle this PDU type.  Send a nack...
                            connection
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
                connection.write_pdu(&response).await?;

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
    connection: Arc<SmppConnection>,
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
    connection: Arc<SmppConnection>,
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

// TODO: split this file into smsc.rs, smpp_connection.rs, smsc_bind.rs,
//       smsc_submit_sm.rs and maybe others.

struct SmppRead {
    stream: ReadHalf<TcpStream>,
    buffer: BytesMut,
}

impl SmppRead {
    async fn read_own_buf(&mut self) -> Result<usize, io::Error> {
        self.stream.read_buf(&mut self.buffer).await
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
}

struct SmppWrite {
    stream: WriteHalf<TcpStream>,
}

impl SmppWrite {}

struct SmppConnection {
    pub socket_addr: SocketAddr,
    // TODO: try std::sync::Mutex instead of tokio::sync - will make disconnect simpler
    read: Mutex<Option<SmppRead>>,
    write: Mutex<Option<SmppWrite>>,
}

impl SmppConnection {
    pub fn new(
        tcp_stream: TcpStream,
        socket_addr: SocketAddr,
    ) -> SmppConnection {
        let (read_stream, write_stream) = split(tcp_stream);
        let buffer = BytesMut::with_capacity(4096);
        let read = SmppRead {
            stream: read_stream,
            buffer,
        };
        let write = SmppWrite {
            stream: write_stream,
        };
        SmppConnection {
            read: Mutex::new(Some(read)),
            write: Mutex::new(Some(write)),
            socket_addr,
        }
    }

    async fn read_pdu(&self) -> Result<Option<Pdu>, PduParseError> {
        loop {
            let mut read = self.read.lock().await;
            if let Some(read) = &mut *read {
                if let Some(pdu) = read.parse_pdu()? {
                    return Ok(Some(pdu));
                }

                if 0 == read.read_own_buf().await? {
                    if read.buffer.is_empty() {
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
    async fn write_pdu(&self, pdu: &Pdu) -> io::Result<()> {
        if let Some(write) = &mut *self.write.lock().await {
            pdu.write(&mut write.stream).await
        } else {
            error!("Attempting to write to a closed connection!");
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    async fn disconnect(&self) {
        self.read.lock().await.take();
        self.write.lock().await.take();
    }
}
