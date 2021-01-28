use ascii::AsciiStr;
use std::io;

use crate::pdu::formats::{COctetString, Integer4, WriteStream};
use crate::pdu::PduParseError;

const MAX_LENGTH_SYSTEM_ID: usize = 16;

#[derive(Debug, PartialEq)]
pub struct BindTransmitterRespPdu {
    pub sequence_number: Integer4,
    pub system_id: COctetString,
}

impl BindTransmitterRespPdu {
    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        let command_length =
            Integer4::new((16 + self.system_id.len() + 1) as u32);
        let command_id = Integer4::new(0x80000002); // bind_transmitter_resp
        let command_status = Integer4::new(0);

        command_length.write(stream).await?;
        command_id.write(stream).await?;
        command_status.write(stream).await?;
        self.sequence_number.write(stream).await?;
        self.system_id.write(stream).await?;

        Ok(())
    }

    pub fn parse(
        _bytes: &mut dyn io::BufRead,
    ) -> Result<BindTransmitterRespPdu, PduParseError> {
        Ok(BindTransmitterRespPdu {
            sequence_number: Integer4::new(0x12),
            system_id: COctetString::new(
                AsciiStr::from_ascii("").unwrap(),
                MAX_LENGTH_SYSTEM_ID,
            ),
        })
    }
}
