use std::io;

use crate::pdu::data::bind_data::BindData;
use crate::pdu::formats::WriteStream;
use crate::pdu::PduParseError;

#[derive(Debug, PartialEq)]
pub struct BindReceiverPdu(BindData);

impl BindReceiverPdu {
    pub fn new(
        system_id: &str,
        password: &str,
        system_type: &str,
        interface_version: u8,
        addr_ton: u8,
        addr_npi: u8,
        address_range: &str,
    ) -> Result<Self, PduParseError> {
        Ok(Self(BindData::new(
            system_id,
            password,
            system_type,
            interface_version,
            addr_ton,
            addr_npi,
            address_range,
        )?))
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        self.0.write(stream).await
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        Ok(Self(BindData::parse(bytes, command_status)?))
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        Ok(Self(self.0.validate_command_status(command_status)?))
    }
}
