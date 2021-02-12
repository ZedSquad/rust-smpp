use std::io;

use crate::pdu::formats::WriteStream;
use crate::pdu::{PduParseError, PduParseErrorBody};

#[derive(Debug, PartialEq)]
pub struct EnquireLinkRespPdu {}

impl EnquireLinkRespPdu {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn write(&self, _stream: &mut WriteStream) -> io::Result<()> {
        Ok(())
    }

    pub fn parse(
        _bytes: &mut dyn io::BufRead,
        _command_status: u32,
    ) -> Result<EnquireLinkRespPdu, PduParseError> {
        Ok(Self {})
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        if command_status == 0x00000000 {
            Ok(self)
        } else {
            Err(PduParseError::new(PduParseErrorBody::StatusIsNotZero))
        }
    }
}
