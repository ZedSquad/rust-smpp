use core::fmt::{Display, Formatter};
use std::error;
use std::io;

use super::formats::OctetStringCreationError;

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

/* TODO: use this PduParseError
pub enum DraftPduParseErrorBody {
    LengthLongerThanPdu(u32),
    LengthTooLong(u32),
    LengthTooShort(u32),
    NotEnoughBytes,
    OctetStringCreationError(OctetStringCreationError),
    OtherIoError(io::Error),
    StatusIsNotZero(u32),
    UnknownCommandId,
}

pub struct DraftPduParseError {
    command_id: Option<u32>,
    sequence_number: Option<u32>,
    field_name: Option<String>,
    body: DraftPduParseErrorBody,
}
*/

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

    pub fn from_octetstringcreationerror(
        e: OctetStringCreationError,
        field_name: &str,
    ) -> PduParseError {
        match e {
            OctetStringCreationError::DoesNotEndWithZeroByte =>
                PduParseError::new(PduParseErrorKind::COctetStringDoesNotEndWithZeroByte, &format!("String value for {} did not end with a zero byte.", field_name), None, None),
            OctetStringCreationError::TooLong(max_len) => PduParseError::new(PduParseErrorKind::COctetStringTooLong, &format!("String value for {} is too long.  Max length is {}, including final zero byte.", field_name, max_len), None, None),
            OctetStringCreationError::NotAscii(e) => PduParseError::new(PduParseErrorKind::COctetStringIsNotAscii, &format!("String value of {} is not ASCII (valid up to byte {}).", field_name, e.valid_up_to()), None, None),
            OctetStringCreationError::OtherIoError(e) => PduParseError::from(e),
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
