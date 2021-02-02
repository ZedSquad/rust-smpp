use ascii::AsciiStr;
use std::io;

use crate::pdu::formats::{COctetString, Integer1, Integer4, WriteStream};
use crate::pdu::{PduParseError, PduParseErrorKind};

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
            system_id: COctetString::new(
                AsciiStr::from_ascii(system_id).map_err(|e| {
                    PduParseError::from_asasciistrerror(e, "system_id")
                })?,
                MAX_LENGTH_SYSTEM_ID,
            )?,
            password: COctetString::new(
                AsciiStr::from_ascii(password).map_err(|e| {
                    PduParseError::from_asasciistrerror(e, "system_id")
                })?,
                MAX_LENGTH_PASSWORD,
            )?,
            system_type: COctetString::new(
                AsciiStr::from_ascii(system_type).map_err(|e| {
                    PduParseError::from_asasciistrerror(e, "system_id")
                })?,
                MAX_LENGTH_SYSTEM_TYPE,
            )?,
            interface_version: Integer1::new(interface_version),
            addr_ton: Integer1::new(addr_ton),
            addr_npi: Integer1::new(addr_npi),
            address_range: COctetString::new(
                AsciiStr::from_ascii(address_range).unwrap(),
                MAX_LENGTH_ADDRESS_RANGE,
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
            return Err(PduParseError {
                kind: PduParseErrorKind::StatusIsNotZero,
                message: format!(
                    "command_status must be 0, but was {}",
                    command_status.value
                ),
                command_id: None,
                io_errorkind: None,
            });
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
