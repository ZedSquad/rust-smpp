use ascii::AsciiString;
use std::io;

use crate::pdu::formats::{COctetString, Integer4, WriteStream};
use crate::pdu::PduParseError;

const MAX_LENGTH_SYSTEM_ID: usize = 16;

#[derive(Debug, PartialEq)]
struct Body {
    pub system_id: COctetString,
}

impl Body {
    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        self.system_id.write(stream).await
    }
}

#[derive(Debug, PartialEq)]
pub struct BindTransmitterRespPdu {
    // command_status: Zero if body is Some; otherwise non-zero
    sequence_number: Integer4,
    body: Option<Body>,
}

impl BindTransmitterRespPdu {
    pub fn new(sequence_number: u32, system_id: &AsciiString) -> Self {
        Self {
            sequence_number: Integer4::new(sequence_number),
            body: Some(Body {
                system_id: COctetString::new(system_id, MAX_LENGTH_SYSTEM_ID),
            }),
        }
    }

    pub fn new_failure(sequence_number: u32) -> Self {
        Self {
            sequence_number: Integer4::new(sequence_number),
            body: None,
        }
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        let command_id = Integer4::new(0x80000002); // bind_transmitter_resp

        if let Some(body) = &self.body {
            let command_length =
                Integer4::new((16 + body.system_id.len() + 1) as u32);
            let command_status = Integer4::new(0x00000000);

            command_length.write(stream).await?;
            command_id.write(stream).await?;
            command_status.write(stream).await?;
            self.sequence_number.write(stream).await?;
            body.write(stream).await?;
        } else {
            let command_length = Integer4::new(16);
            let command_status = Integer4::new(0x00000001);

            command_length.write(stream).await?;
            command_id.write(stream).await?;
            command_status.write(stream).await?;
            self.sequence_number.write(stream).await?;
            // We don't write the body when status is non-zero
        }

        Ok(())
    }

    pub fn parse(
        _bytes: &mut dyn io::BufRead,
    ) -> Result<BindTransmitterRespPdu, PduParseError> {
        todo!("BindTransmitterRespPdu::parse");
    }
}
