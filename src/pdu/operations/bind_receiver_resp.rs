use std::io;

use crate::pdu::data::bind_resp_data::BindRespData;
use crate::pdu::formats::WriteStream;
use crate::pdu::PduParseError;

#[derive(Debug, PartialEq)]
pub struct BindReceiverRespPdu(BindRespData);

impl BindReceiverRespPdu {
    pub fn new(system_id: &str) -> Result<Self, PduParseError> {
        Ok(Self(BindRespData::new(system_id)?))
    }

    pub fn new_error() -> Self {
        Self(BindRespData::new_error())
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        self.0.write(stream).await
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        Ok(Self(BindRespData::parse(bytes, command_status)?))
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        Ok(Self(self.0.validate_command_status(command_status)?))
    }
}
