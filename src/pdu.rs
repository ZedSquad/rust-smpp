use std::convert::TryFrom;
use std::io::{BufRead, Cursor, ErrorKind};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::result::Result;

// https://smpp.org/smppv34_gsmumts_ig_v10.pdf p11 states:
// "... message_payload parameter which can hold up to a maximum of 64K ..."
// So we guess no valid PDU can be longer than 70K octets.
const MAX_PDU_LENGTH: u32 = 70000;

#[derive(Debug)]
pub enum Pdu {
    BindTransmitter(BindTransmitterPdu),
    BindTransmitterResp(BindTransmitterRespPdu),
}

#[derive(Debug, PartialEq)]
pub enum CheckOutcome {
    Ok,
    Incomplete,
}

impl Pdu {
    pub fn parse(bytes: &mut Cursor<&[u8]>) -> Result<Pdu> {
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

    pub fn check(bytes: &mut dyn BufRead) -> Result<CheckOutcome> {
        check(bytes)
    }

    pub async fn write(&self, tcp_stream: &mut TcpStream) -> Result<()> {
        match self {
            Pdu::BindTransmitter(pdu) => pdu.write(tcp_stream).await,
            Pdu::BindTransmitterResp(pdu) => pdu.write(tcp_stream).await,
        }
    }
}

fn check(bytes: &mut dyn BufRead) -> Result<CheckOutcome> {
    let mut len: [u8; 4] = [0; 4];
    bytes.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len);

    if len > MAX_PDU_LENGTH {
        return Err(format!(
            "PDU too long!  Length: {}, max allowed: {}",
            len, MAX_PDU_LENGTH
        )
        .into());
    }

    check_can_read(bytes, len - 4)
}

fn check_can_read(bytes: &mut dyn BufRead, len: u32) -> Result<CheckOutcome> {
    let len = usize::try_from(len)?;
    // Is there a better way than allocating this vector?
    let mut buf = Vec::with_capacity(len);
    buf.resize(len, 0);
    bytes
        .read_exact(buf.as_mut_slice())
        .map(|_| CheckOutcome::Ok)
        .or_else(|e| match e.kind() {
            ErrorKind::UnexpectedEof => Ok(CheckOutcome::Incomplete),
            _ => Err(e.into()),
        })
}

#[derive(Debug)]
pub struct BindTransmitterPdu {
    pub sequence_number: u32,
    pub system_id: String,
    pub password: String,
    pub system_type: String,
    pub interface_version: u8,
    pub addr_ton: u8,
    pub addr_npi: u8,
    pub address_range: String,
}

impl BindTransmitterPdu {
    async fn write(&self, _tcp_stream: &mut TcpStream) -> Result<()> {
        todo!()
    }
}

#[derive(Debug)]
pub struct BindTransmitterRespPdu {
    pub sequence_number: u32,
    pub system_id: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    const BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA: &[u8; 0x1b + 0xa] =
        b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02TestServer\0extrabytes";

    #[test]
    fn check_is_ok_if_more_bytes() {
        let mut cursor = Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..]);
        assert_eq!(Pdu::check(&mut cursor).unwrap(), CheckOutcome::Ok);
    }

    #[test]
    fn check_is_ok_if_exact_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1b]);
        assert_eq!(Pdu::check(&mut cursor).unwrap(), CheckOutcome::Ok);
    }

    #[test]
    fn check_is_incomplete_if_fewer_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1a]);
        assert_eq!(Pdu::check(&mut cursor).unwrap(), CheckOutcome::Incomplete);
    }

    #[test]
    fn check_errors_if_read_error() {
        fn e() -> io::Error {
            io::Error::from_raw_os_error(22)
        }
        struct FailingRead {}
        impl io::Read for FailingRead {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(e())
            }
        }
        let mut failing_read = io::BufReader::new(FailingRead {});
        let res = Pdu::check(&mut failing_read).map_err(|e| e.to_string());
        assert_eq!(res, Err(e().to_string()));
    }
}
