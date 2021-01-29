use std::io;

use crate::pdu::formats::{Integer4, WriteStream};
use crate::pdu::PduParseError;

pub const GENERIC_NACK: u32 = 0x80000000;

#[derive(Debug, PartialEq)]
pub struct GenericNackPdu {
    command_status: Integer4,
    sequence_number: Integer4,
}

impl GenericNackPdu {
    pub fn new(command_status: u32, sequence_number: u32) -> Self {
        Self {
            command_status: Integer4::new(command_status),
            sequence_number: Integer4::new(sequence_number),
        }
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        let command_length = Integer4::new(16);
        let command_id = Integer4::new(GENERIC_NACK);

        command_length.write(stream).await?;
        command_id.write(stream).await?;
        self.command_status.write(stream).await?;
        self.sequence_number.write(stream).await?;

        Ok(())
    }

    pub fn parse(_bytes: &mut dyn io::BufRead) -> Result<Self, PduParseError> {
        todo!("GenericNackPdu::parse");
    }
}
