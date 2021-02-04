use std::io;

use crate::pdu::formats::{
    COctetString, Integer1, Integer4, OctetStringCreationError, WriteStream,
};
use crate::pdu::PduParseError;

const MAX_LENGTH_SYSTEM_ID: usize = 16;
const MAX_LENGTH_PASSWORD: usize = 9;
const MAX_LENGTH_SYSTEM_TYPE: usize = 13;
const MAX_LENGTH_ADDRESS_RANGE: usize = 41;

#[derive(Debug, PartialEq)]
pub struct BindTransmitterPdu {
    pub sequence_number: Integer4,
    system_id: COctetString,
    password: COctetString,
    system_type: COctetString,
    interface_version: Integer1,
    addr_ton: Integer1,
    addr_npi: Integer1,
    address_range: COctetString,
}

fn map_e(
    res: Result<COctetString, OctetStringCreationError>,
    field_name: &str,
) -> Result<COctetString, PduParseError> {
    res.map_err(|e| PduParseError::from(e).into_with_field_name(field_name))
}

impl BindTransmitterPdu {
    pub fn new(
        sequence_number: u32,
        system_id: &str,
        password: &str,
        system_type: &str,
        interface_version: u8,
        addr_ton: u8,
        addr_npi: u8,
        address_range: &str,
    ) -> Result<Self, PduParseError> {
        Ok(Self {
            sequence_number: Integer4::new(sequence_number),
            system_id: map_e(
                COctetString::from_str(system_id, MAX_LENGTH_SYSTEM_ID),
                "system_id",
            )?,
            password: map_e(
                COctetString::from_str(password, MAX_LENGTH_PASSWORD),
                "password",
            )?,
            system_type: map_e(
                COctetString::from_str(system_type, MAX_LENGTH_SYSTEM_TYPE),
                "system_type",
            )?,
            interface_version: Integer1::new(interface_version),
            addr_ton: Integer1::new(addr_ton),
            addr_npi: Integer1::new(addr_npi),
            address_range: map_e(
                COctetString::from_str(address_range, MAX_LENGTH_ADDRESS_RANGE),
                "address_range",
            )?,
        })
    }

    pub async fn write(&self, _stream: &mut WriteStream) -> io::Result<()> {
        todo!()
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
    ) -> Result<BindTransmitterPdu, PduParseError> {
        let command_status = Integer4::read(bytes)?;
        let sequence_number = Integer4::read(bytes)?;
        let system_id = map_e(
            COctetString::read(bytes, MAX_LENGTH_SYSTEM_ID),
            "system_id",
        )?;
        let password =
            map_e(COctetString::read(bytes, MAX_LENGTH_PASSWORD), "password")?;
        let system_type = map_e(
            COctetString::read(bytes, MAX_LENGTH_SYSTEM_TYPE),
            "system_type",
        )?;
        let interface_version = Integer1::read(bytes)?;
        let addr_ton = Integer1::read(bytes)?;
        let addr_npi = Integer1::read(bytes)?;
        let address_range = map_e(
            COctetString::read(bytes, MAX_LENGTH_ADDRESS_RANGE),
            "address_range",
        )?;

        if command_status.value != 0x00 {
            return Err(PduParseError::for_statusisnotzero(
                command_status.value,
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
