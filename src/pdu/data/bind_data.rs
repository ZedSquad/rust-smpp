use std::io;

use crate::pdu::formats::{COctetString, Integer1, WriteStream};
use crate::pdu::pduparseerror::fld;
use crate::pdu::{PduParseError, PduParseErrorBody};

const MAX_LENGTH_SYSTEM_ID: usize = 16;
const MAX_LENGTH_PASSWORD: usize = 9;
const MAX_LENGTH_SYSTEM_TYPE: usize = 13;
const MAX_LENGTH_ADDRESS_RANGE: usize = 41;

#[derive(Debug, PartialEq)]
pub struct BindData {
    system_id: COctetString,
    password: COctetString,
    system_type: COctetString,
    interface_version: Integer1,
    addr_ton: Integer1,
    addr_npi: Integer1,
    address_range: COctetString,
}

impl BindData {
    pub fn new(
        system_id: &str,
        password: &str,
        system_type: &str,
        interface_version: u8,
        addr_ton: u8,
        addr_npi: u8,
        address_range: &str,
    ) -> Result<Self, PduParseError> {
        Ok(Self {
            system_id: fld(
                "system_id",
                COctetString::from_str(system_id, MAX_LENGTH_SYSTEM_ID),
            )?,
            password: fld(
                "password",
                COctetString::from_str(password, MAX_LENGTH_PASSWORD),
            )?,
            system_type: fld(
                "system_type",
                COctetString::from_str(system_type, MAX_LENGTH_SYSTEM_TYPE),
            )?,
            interface_version: Integer1::new(interface_version),
            addr_ton: Integer1::new(addr_ton),
            addr_npi: Integer1::new(addr_npi),
            address_range: fld(
                "address_range",
                COctetString::from_str(address_range, MAX_LENGTH_ADDRESS_RANGE),
            )?,
        })
    }

    pub async fn write(&self, _stream: &mut WriteStream) -> io::Result<()> {
        todo!()
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
        _command_status: u32,
    ) -> Result<Self, PduParseError> {
        let system_id =
            fld("system_id", COctetString::read(bytes, MAX_LENGTH_SYSTEM_ID))?;
        let password =
            fld("password", COctetString::read(bytes, MAX_LENGTH_PASSWORD))?;
        let system_type = fld(
            "system_type",
            COctetString::read(bytes, MAX_LENGTH_SYSTEM_TYPE),
        )?;
        let interface_version =
            fld("interface_version", Integer1::read(bytes))?;
        let addr_ton = fld("addr_ton", Integer1::read(bytes))?;
        let addr_npi = fld("addr_npi", Integer1::read(bytes))?;
        let address_range = fld(
            "address_range",
            COctetString::read(bytes, MAX_LENGTH_ADDRESS_RANGE),
        )?;

        Ok(Self {
            system_id,
            password,
            system_type,
            interface_version,
            addr_ton,
            addr_npi,
            address_range,
        })
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
