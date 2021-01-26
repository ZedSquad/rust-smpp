use std::io;

use crate::pdu::PduParseErrorKind;

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
