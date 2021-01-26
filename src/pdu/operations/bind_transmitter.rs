use std::io;

use crate::pdu::formats::{COctetString, Integer1, Integer4, WriteStream};
use crate::result::Result;

pub const MAX_LENGTH_SYSTEM_ID: usize = 16;
pub const MAX_LENGTH_PASSWORD: usize = 9;
pub const MAX_LENGTH_SYSTEM_TYPE: usize = 13;
pub const MAX_LENGTH_ADDRESS_RANGE: usize = 41;

#[derive(Debug, PartialEq)]
pub struct BindTransmitterPdu {
    pub sequence_number: Integer4,
    pub system_id: COctetString,
    pub password: COctetString,
    pub system_type: COctetString,
    pub interface_version: Integer1,
    pub addr_ton: Integer1,
    pub addr_npi: Integer1,
    pub address_range: COctetString,
}

impl BindTransmitterPdu {
    pub async fn write(&self, _tcp_stream: &mut WriteStream) -> Result<()> {
        todo!()
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
    ) -> io::Result<BindTransmitterPdu> {
        let command_status = Integer4::read(bytes)?;
        let sequence_number = Integer4::read(bytes)?;
        let system_id =
            COctetString::read(bytes, MAX_LENGTH_SYSTEM_ID, "system_id")?;
        let password =
            COctetString::read(bytes, MAX_LENGTH_PASSWORD, "password")?;
        let system_type =
            COctetString::read(bytes, MAX_LENGTH_SYSTEM_TYPE, "system_type")?;
        let interface_version = Integer1::read(bytes)?;
        let addr_ton = Integer1::read(bytes)?;
        let addr_npi = Integer1::read(bytes)?;
        let address_range = COctetString::read(
            bytes,
            MAX_LENGTH_ADDRESS_RANGE,
            "address_range",
        )?;

        if command_status.value != 0x00 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "command_status must be 0, but was {}",
                    command_status.value
                ),
            ));
        }

        Ok(BindTransmitterPdu {
            sequence_number,
            system_id,
            password,
            system_type,
            interface_version,
            addr_ton,
            addr_npi,
            address_range,
        })
    }
}
