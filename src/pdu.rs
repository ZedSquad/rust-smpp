use ascii::AsciiStr;
use std::convert::TryFrom;
use std::io::{BufRead, Cursor, ErrorKind, Read};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::pdu_types::{Integer1, Integer4, ReadSelf};
use crate::result::Result;

// https://smpp.org/smppv34_gsmumts_ig_v10.pdf p11 states:
// "... message_payload parameter which can hold up to a maximum of 64K ..."
// So we guess no valid PDU can be longer than 70K octets.
const MAX_PDU_LENGTH: u32 = 70000;

// We need at least a command_length and command_id, so 8 bytes
const MIN_PDU_LENGTH: u32 = 8;

#[derive(Debug, PartialEq)]
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
        let _command_length = Integer4::read_self(bytes)?;
        let command_id = Integer4::read_self(bytes)?;

        match command_id.value {
            0x00000002 => BindTransmitterPdu::parse(bytes)
                .map(|p| Pdu::BindTransmitter(p)),
            0x80000002 => BindTransmitterRespPdu::parse(bytes)
                .map(|p| Pdu::BindTransmitterResp(p)),
            _ => {
                Err(format!("Unknown command id: {}", command_id.value).into())
            }
        }
    }

    pub fn check(bytes: &mut dyn Read) -> Result<CheckOutcome> {
        check(bytes)
    }

    pub async fn write(&self, tcp_stream: &mut TcpStream) -> Result<()> {
        match self {
            Pdu::BindTransmitter(pdu) => pdu.write(tcp_stream).await,
            Pdu::BindTransmitterResp(pdu) => pdu.write(tcp_stream).await,
        }
    }
}

fn check(bytes: &mut dyn Read) -> Result<CheckOutcome> {
    let command_length = Integer4::read_self(bytes);
    match command_length {
        Ok(command_length) => check_can_read(bytes, command_length.value),
        Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => Ok(CheckOutcome::Incomplete),
            _ => Err(e.into()),
        },
    }
}

fn check_can_read(
    bytes: &mut dyn Read,
    command_length: u32,
) -> Result<CheckOutcome> {
    if command_length > MAX_PDU_LENGTH {
        return Err(format!(
            "PDU too long!  Length: {}, max allowed: {}",
            command_length, MAX_PDU_LENGTH
        )
        .into());
    } else if command_length < MIN_PDU_LENGTH {
        return Err(format!(
            "PDU too short!  Length: {}, min allowed: {}",
            command_length, MIN_PDU_LENGTH
        )
        .into());
    }

    let len = usize::try_from(command_length - 4)?;
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

#[derive(Debug, PartialEq)]
pub struct BindTransmitterPdu {
    pub sequence_number: Integer4,
    pub system_id: String,
    pub password: String,
    pub system_type: String,
    pub interface_version: Integer1,
    pub addr_ton: Integer1,
    pub addr_npi: Integer1,
    pub address_range: String,
}

impl BindTransmitterPdu {
    async fn write(&self, _tcp_stream: &mut TcpStream) -> Result<()> {
        todo!()
    }

    fn parse(bytes: &mut Cursor<&[u8]>) -> Result<BindTransmitterPdu> {
        let command_status = Integer4::read_self(bytes)?;
        let sequence_number = Integer4::read_self(bytes)?;
        let system_id = read_c_octet_string(bytes, 16, "system_id")?;
        let password = read_c_octet_string(bytes, 9, "password")?;
        let system_type = read_c_octet_string(bytes, 13, "system_type")?;
        let interface_version = Integer1::read_self(bytes)?;
        let addr_ton = Integer1::read_self(bytes)?;
        let addr_npi = Integer1::read_self(bytes)?;
        let address_range = read_c_octet_string(bytes, 41, "address_range")?;

        if command_status.value != 0x00 {
            return Err(format!(
                "command_status must be 0, but was {}",
                command_status.value
            )
            .into());
        }

        Ok(BindTransmitterPdu {
            sequence_number,
            system_id,
            password,
            system_type,
            interface_version,
            addr_ton,
            addr_npi,
            address_range,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct BindTransmitterRespPdu {
    pub sequence_number: Integer4,
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
        tcp_stream.write_u32(self.sequence_number.value).await?;
        // TODO: check allowed characters (on creation and/or here)
        tcp_stream.write_all(self.system_id.as_bytes()).await?;
        tcp_stream.write_u8(0x00).await?;
        Ok(())
    }

    fn parse(_bytes: &mut Cursor<&[u8]>) -> Result<BindTransmitterRespPdu> {
        Ok(BindTransmitterRespPdu {
            sequence_number: Integer4::new(0x12),
            system_id: String::from(""),
        })
    }
}

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 3.1
///
/// C-Octet String:
/// A series of ASCII characters terminated with the NULL character.
fn read_c_octet_string(
    bytes: &mut dyn BufRead,
    max_len: u64,
    field_name: &str,
) -> Result<String> {
    let mut buf = Vec::new();
    bytes.take(max_len).read_until(0x00, &mut buf)?;

    if buf.last() != Some(&0x00) {
        // Failed to read a NULL terminator before we ran out of characters
        return Err(
            format!("String value for {} was too long", field_name).into()
        );
    }

    let buf = &buf[..(buf.len() - 1)]; // Remove trailing 0 byte
    AsciiStr::from_ascii(buf)
        .map(|s| String::from(s.as_str()))
        .map_err(|e| e.into())
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

    #[test]
    fn parse_valid_bind_transmitter() {
        const BIND_TRANSMITTER_PDU_PLUS_EXTRA: &[u8; 0x2e + 0x6] =
            b"\x00\x00\x00\x2e\x00\x00\x00\x02\x00\x00\x00\x00\x01\x02\x03\x44mysystem_ID\0pw$xx\0t_p_\0\x34\x13\x50rng\0foobar";

        let mut cursor = Cursor::new(&BIND_TRANSMITTER_PDU_PLUS_EXTRA[..]);
        assert_eq!(
            Pdu::parse(&mut cursor).unwrap(),
            Pdu::BindTransmitter(BindTransmitterPdu {
                sequence_number: Integer4::new(0x01020344),
                system_id: String::from("mysystem_ID"),
                password: String::from("pw$xx"),
                system_type: String::from("t_p_"),
                interface_version: Integer1::new(0x34),
                addr_ton: Integer1::new(0x13),
                addr_npi: Integer1::new(0x50),
                address_range: String::from("rng"),
            })
        );
    }

    // TODO: variable-length c-octet strings that are longer than max length
    // TODO: max length INCLUDES the NULL character
    // TODO: very long strings inside PDU with short length
    // TODO: long length, short pdu
    // TODO: long length, long pdu
    // TODO: non-ascii characters in c-octet string
}
