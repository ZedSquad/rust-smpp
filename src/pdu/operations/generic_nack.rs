use std::io;

use crate::pdu::formats::WriteStream;
use crate::pdu::{PduParseError, PduParseErrorBody};

// TODO: no need for this struct at all?
#[derive(Debug, PartialEq)]
pub struct GenericNackPdu {}

impl GenericNackPdu {
    pub fn new_error() -> Self {
        Self {}
    }

    pub async fn write(&self, _stream: &mut WriteStream) -> io::Result<()> {
        Ok(())
    }

    pub fn parse(
        _bytes: &mut dyn io::BufRead,
        _command_status: u32,
    ) -> Result<Self, PduParseError> {
        todo!("GenericNackPdu::parse");
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        if command_status == 0 {
            Err(PduParseError::new(PduParseErrorBody::StatusIsZero))
        } else {
            Ok(self)
        }
    }
}
