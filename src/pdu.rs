use ascii::AsciiStr;
use std::convert::TryFrom;
use std::io;
use std::io::{BufRead, ErrorKind, Read};

use crate::pdu_types::{COctetString, Integer1, Integer4, WriteStream};
use crate::result::Result;

// https://smpp.org/smppv34_gsmumts_ig_v10.pdf p11 states:
// "... message_payload parameter which can hold up to a maximum of 64K ..."
// So we guess no valid PDU can be longer than 70K octets.
const MAX_PDU_LENGTH: usize = 70000;

// We need at least a command_length and command_id, so 8 bytes
const MIN_PDU_LENGTH: usize = 8;

pub const MAX_LENGTH_SYSTEM_ID: usize = 16;
const MAX_LENGTH_PASSWORD: usize = 9;
const MAX_LENGTH_SYSTEM_TYPE: usize = 13;
const MAX_LENGTH_ADDRESS_RANGE: usize = 41;

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
    pub fn parse(bytes: &mut dyn BufRead) -> io::Result<Pdu> {
        let command_length = Integer4::read(bytes)?;
        validate_command_length(&command_length)?;
        let mut bytes =
            bytes.take(u64::try_from(command_length.value - 4).unwrap_or(0));
        let command_id = Integer4::read(&mut bytes)?;

        match command_id.value {
            0x00000002 => BindTransmitterPdu::parse(&mut bytes)
                .map(|p| Pdu::BindTransmitter(p)),
            0x80000002 => BindTransmitterRespPdu::parse(&mut bytes)
                .map(|p| Pdu::BindTransmitterResp(p)),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unknown command id: {}", command_id.value),
            )),
        }.map_err(|e| match e.kind() {
            io::ErrorKind::UnexpectedEof =>
                io::Error::new(
                    io::ErrorKind::Other,
                    "Reached end of PDU length (or end of input) before finding all fields of the PDU."
                ),
            _ => e
        })
    }

    pub fn check(bytes: &mut dyn BufRead) -> Result<CheckOutcome> {
        check(bytes)
    }

    pub async fn write(&self, tcp_stream: &mut WriteStream) -> Result<()> {
        match self {
            Pdu::BindTransmitter(pdu) => pdu.write(tcp_stream).await,
            Pdu::BindTransmitterResp(pdu) => pdu.write(tcp_stream).await,
        }
    }
}

fn check(bytes: &mut dyn BufRead) -> Result<CheckOutcome> {
    let command_length = Integer4::read(bytes);
    match command_length {
        Ok(command_length) => {
            validate_command_length(&command_length)?;
            check_can_read(bytes, command_length.value)
        }
        Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => Ok(CheckOutcome::Incomplete),
            _ => Err(e.into()),
        },
    }
}

fn validate_command_length(command_length: &Integer4) -> io::Result<()> {
    let len = command_length.value as usize;
    if len > MAX_PDU_LENGTH {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "PDU too long!  Length: {}, max allowed: {}",
                len, MAX_PDU_LENGTH
            ),
        ))
    } else if len < MIN_PDU_LENGTH {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "PDU too short!  Length: {}, min allowed: {}",
                len, MIN_PDU_LENGTH
            ),
        ))
    } else {
        Ok(())
    }
}

fn check_can_read(
    bytes: &mut dyn BufRead,
    command_length: u32,
) -> Result<CheckOutcome> {
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
    pub system_id: COctetString,
    pub password: COctetString,
    pub system_type: COctetString,
    pub interface_version: Integer1,
    pub addr_ton: Integer1,
    pub addr_npi: Integer1,
    pub address_range: COctetString,
}

impl BindTransmitterPdu {
    async fn write(&self, _tcp_stream: &mut WriteStream) -> Result<()> {
        todo!()
    }

    fn parse(bytes: &mut dyn BufRead) -> io::Result<BindTransmitterPdu> {
        let command_status = Integer4::read(bytes)?;
        let sequence_number = Integer4::read(bytes)?;
        let system_id =
            COctetString::read(bytes, MAX_LENGTH_SYSTEM_ID, "system_id")?;
        let password =
            COctetString::read(bytes, MAX_LENGTH_PASSWORD, "password")?;
        let system_type =
            COctetString::read(bytes, MAX_LENGTH_SYSTEM_TYPE, "system_type")?;
        let interface_version = Integer1::read(bytes)?;
        let addr_ton = Integer1::read(bytes)?;
        let addr_npi = Integer1::read(bytes)?;
        let address_range = COctetString::read(
            bytes,
            MAX_LENGTH_ADDRESS_RANGE,
            "address_range",
        )?;

        if command_status.value != 0x00 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "command_status must be 0, but was {}",
                    command_status.value
                ),
            ));
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
    pub system_id: COctetString,
}

impl BindTransmitterRespPdu {
    async fn write(&self, tcp_stream: &mut WriteStream) -> Result<()> {
        let command_length =
            Integer4::new((16 + self.system_id.len() + 1) as u32);
        let command_id = Integer4::new(0x80000002); // bind_transmitter_resp
        let command_status = Integer4::new(0);

        command_length.write(tcp_stream).await?;
        command_id.write(tcp_stream).await?;
        command_status.write(tcp_stream).await?;
        self.sequence_number.write(tcp_stream).await?;
        self.system_id.write(tcp_stream).await?;

        Ok(())
    }

    fn parse(_bytes: &mut dyn BufRead) -> io::Result<BindTransmitterRespPdu> {
        Ok(BindTransmitterRespPdu {
            sequence_number: Integer4::new(0x12),
            system_id: COctetString::new(
                AsciiStr::from_ascii("").unwrap(),
                MAX_LENGTH_SYSTEM_ID,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unittest_utils::FailingRead;
    use std::io::Cursor;

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
        let mut failing_read = FailingRead::new_bufreader();
        let res = Pdu::check(&mut failing_read).unwrap_err();
        assert_eq!(res.to_string(), FailingRead::error_string());
    }

    #[test]
    fn check_errors_without_reading_all_if_long_length() {
        const PDU: &[u8; 16] =
            b"\xff\xff\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00";
        let mut cursor = Cursor::new(&PDU);

        let res = Pdu::check(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "PDU too long!  Length: 4294967295, max allowed: 70000"
        );
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
                system_id: COctetString::new(
                    AsciiStr::from_ascii("mysystem_ID").unwrap(),
                    MAX_LENGTH_SYSTEM_ID
                ),
                password: COctetString::new(
                    AsciiStr::from_ascii("pw$xx").unwrap(),
                    MAX_LENGTH_PASSWORD
                ),
                system_type: COctetString::new(
                    AsciiStr::from_ascii("t_p_").unwrap(),
                    MAX_LENGTH_SYSTEM_TYPE
                ),
                interface_version: Integer1::new(0x34),
                addr_ton: Integer1::new(0x13),
                addr_npi: Integer1::new(0x50),
                address_range: COctetString::new(
                    AsciiStr::from_ascii("rng").unwrap(),
                    MAX_LENGTH_ADDRESS_RANGE
                ),
            })
        );
    }

    #[test]
    fn parse_bind_transmitter_with_too_long_system_id() {
        const PDU: &[u8; 0x29] =
            b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x01\x02\x03\x44ABDEFABCDEFABCDEFA\0\0\0\x34\x13\x50\0";
        let mut cursor = Cursor::new(&PDU[..]);

        let res = Pdu::parse(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "String value for system_id is too long.  Max length is 16, including final zero byte."
        );
    }

    #[test]
    fn parse_bind_transmitter_with_length_ending_within_string() {
        const PDU: &[u8; 0x29] =
            b"\x00\x00\x00\x12\x00\x00\x00\x02\x00\x00\x00\x00\x01\x02\x03\x44ABDEFABCDEFABCDEFA\0\0\0\x34\x13\x50\0";
        let mut cursor = Cursor::new(&PDU[..]);

        let res = Pdu::parse(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "String value for system_id did not end with a zero byte."
        );
    }

    #[test]
    fn parse_bind_transmitter_ending_before_all_fields() {
        const PDU: &[u8; 0x13] =
            b"\x00\x00\x00\x13\x00\x00\x00\x02\x00\x00\x00\x00\x01\x02\x03\x44\0\0\0";
        let mut cursor = Cursor::new(&PDU[..]);

        let res = Pdu::parse(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "Reached end of PDU length (or end of input) before finding all fields of the PDU."
        );
    }

    #[test]
    fn parse_bind_transmitter_hitting_eof_before_end_of_length() {
        const PDU: &[u8; 0x0b] =
            b"\x00\x00\x00\x2e\x00\x00\x00\x02\x00\x00\x00";
        let mut cursor = Cursor::new(&PDU[..]);

        let res = Pdu::parse(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "Reached end of PDU length (or end of input) before finding all fields of the PDU."
        );
    }

    #[test]
    fn parse_bind_transmitter_with_massive_length() {
        const PDU: &[u8; 16] =
            b"\xff\xff\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00";
        let mut cursor = Cursor::new(&PDU);

        let res = Pdu::parse(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "PDU too long!  Length: 4294967295, max allowed: 70000"
        );
    }

    // TODO: non-ascii characters in c-octet string
}
