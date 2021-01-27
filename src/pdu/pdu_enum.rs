use core::fmt::{Display, Formatter};
use std::convert::TryFrom;
use std::error::Error;
use std::io;
use std::io::Read;

use crate::pdu::formats::{Integer4, WriteStream};
use crate::pdu::validate_command_length::validate_command_length;
use crate::pdu::{check, BindTransmitterPdu, BindTransmitterRespPdu};
use crate::pdu::{CheckError, CheckOutcome};
use crate::result;

#[derive(Debug, PartialEq)]
pub enum PduParseErrorKind {
    LengthTooLong,
    LengthTooShort,
    NonasciiCOctetString,
    NotEnoughBytes,
    OtherIoError,
}

#[derive(Debug, PartialEq)]
pub struct PduParseError {
    pub kind: PduParseErrorKind,
    pub message: String,
    pub command_id: Option<u32>,
    pub io_errorkind: Option<io::ErrorKind>,
}

impl PduParseError {
    pub fn new(
        kind: PduParseErrorKind,
        message: &str,
        command_id: Option<u32>,
        io_errorkind: Option<io::ErrorKind>,
    ) -> PduParseError {
        PduParseError {
            kind,
            message: String::from(message),
            command_id,
            io_errorkind,
        }
    }

    fn from_io_error_with_command_id(
        e: io::Error,
        command_id: Option<u32>,
    ) -> Self {
        let (kind, message) = match e.kind() {
            io::ErrorKind::UnexpectedEof => (
                PduParseErrorKind::NotEnoughBytes,
                String::from("Reached end of PDU length (or end of input) before finding all fields of the PDU.")
            ),
            _ => (
                PduParseErrorKind::OtherIoError,
                e.to_string()
            ),
        };
        Self {
            kind,
            message,
            command_id,
            io_errorkind: Some(e.kind()),
        }
    }
}

impl From<io::Error> for PduParseError {
    fn from(e: io::Error) -> Self {
        Self::from_io_error_with_command_id(e, None)
    }
}

impl Display for PduParseError {
    fn fmt(
        &self,
        formatter: &mut Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        let command_id = self
            .command_id
            .map(|id| format!("{:#08X}", id))
            .unwrap_or(String::from("UNKNOWN"));
        if let Some(ek) = self.io_errorkind {
            formatter.write_fmt(format_args!(
                "Error parsing PDU: {}. (command_id={}, PduParseErrorKind={:?}, io::ErrorKind={:?})",
                self.message, command_id, self.kind, ek
            ))
        } else {
            formatter.write_fmt(format_args!(
                "Error parsing PDU: {}. (command_id={}, PduParseErrorKind={:?})",
                self.message, command_id, self.kind
            ))
        }
    }
}

impl Error for PduParseError {}

#[derive(Debug, PartialEq)]
pub enum Pdu {
    BindTransmitter(BindTransmitterPdu),
    BindTransmitterResp(BindTransmitterRespPdu),
}

fn pe(command_id: u32) -> Box<dyn FnOnce(io::Error) -> PduParseError> {
    Box::new(move |e| {
        PduParseError::from_io_error_with_command_id(e, Some(command_id))
    })
}

impl Pdu {
    pub fn parse(bytes: &mut dyn io::BufRead) -> Result<Pdu, PduParseError> {
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
        }
        .map_err(pe(command_id.value))
    }

    pub fn check(
        bytes: &mut dyn io::BufRead,
    ) -> Result<CheckOutcome, CheckError> {
        check::check(bytes)
    }

    // TODO: return io::Result from write()?
    pub async fn write(
        &self,
        tcp_stream: &mut WriteStream,
    ) -> result::Result<()> {
        match self {
            Pdu::BindTransmitter(pdu) => pdu.write(tcp_stream).await,
            Pdu::BindTransmitterResp(pdu) => pdu.write(tcp_stream).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use ascii::AsciiStr;
    use std::io::Cursor;

    use super::*;
    use crate::pdu::formats::{COctetString, Integer1};
    use crate::pdu::operations::bind_transmitter;

    const BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA: &[u8; 0x1b + 0xa] =
        b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02TestServer\0extrabytes";

    #[test]
    fn check_is_ok_if_more_bytes() {
        // Most tests for check are in the check module.  Here we do enough
        // to confirm that we are calling through it that from Pdu::check.
        let mut cursor = Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..]);
        assert_eq!(Pdu::check(&mut cursor).unwrap(), CheckOutcome::Ready);
    }

    #[test]
    fn check_is_incomplete_if_fewer_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1a]);
        assert_eq!(Pdu::check(&mut cursor).unwrap(), CheckOutcome::Incomplete);
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
                    bind_transmitter::MAX_LENGTH_SYSTEM_ID
                ),
                password: COctetString::new(
                    AsciiStr::from_ascii("pw$xx").unwrap(),
                    bind_transmitter::MAX_LENGTH_PASSWORD
                ),
                system_type: COctetString::new(
                    AsciiStr::from_ascii("t_p_").unwrap(),
                    bind_transmitter::MAX_LENGTH_SYSTEM_TYPE
                ),
                interface_version: Integer1::new(0x34),
                addr_ton: Integer1::new(0x13),
                addr_npi: Integer1::new(0x50),
                address_range: COctetString::new(
                    AsciiStr::from_ascii("rng").unwrap(),
                    bind_transmitter::MAX_LENGTH_ADDRESS_RANGE
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
    fn parse_bind_transmitter_with_short_length() {
        const PDU: &[u8; 4] = b"\x00\x00\x00\x04";
        let mut cursor = Cursor::new(&PDU);

        let res = Pdu::parse(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "PDU too short!  Length: 4, min allowed: 8."
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
            "PDU too long!  Length: 4294967295, max allowed: 70000."
        );
    }

    /*#[test]
    fn parse_bind_transmitter_containing_nonascii_characters() {
        const PDU: &[u8; 0x2e + 0x6] =
            b"\x00\x00\x00\x2e\x00\x00\x00\x02\x00\x00\x00\x00\x01\x02\x03\x44mys\xf0\x9f\x92\xa9m_ID\0pw$xx\0t_p_\0\x34\x13\x50rng\0foobar";
        let mut cursor = Cursor::new(&PDU);

        let res = Pdu::parse(&mut cursor).unwrap_err();
        assert_eq!(
            res,
            PduParseError::new(
                PduParseErrorKind::NonasciiCOctetString,
                "String value of system_id is not ASCII (valid up to byte 3).",
                Some(0x00000002),
                None,
            )
        );
    }*/
}
