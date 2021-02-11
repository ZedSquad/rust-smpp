use std::io;

use crate::pdu::formats::WriteStream;
use crate::pdu::PduParseError;

pub const GENERIC_NACK: u32 = 0x80000000;

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
}
