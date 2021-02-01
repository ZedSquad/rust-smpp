use core::fmt::{Display, Formatter};
use std::error;
use std::io;

#[derive(Debug, PartialEq)]
pub enum PduParseErrorKind {
    LengthLongerThanPdu,
    LengthTooLong,
    LengthTooShort,
    COctetStringDoesNotEndWithZeroByte,
    COctetStringIsNotAscii,
    COctetStringTooLong,
    NotEnoughBytes,
    OtherIoError,
    StatusIsNotZero,
    UnknownCommandId,
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
}

impl From<io::Error> for PduParseError {
    fn from(e: io::Error) -> Self {
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
            command_id: None,
            io_errorkind: Some(e.kind()),
        }
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

impl error::Error for PduParseError {}
