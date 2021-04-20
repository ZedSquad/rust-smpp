use std::io;

use crate::pdu::data::sm_data::SmData;
use crate::pdu::formats::WriteStream;
use crate::pdu::tlvs::Tlvs;
use crate::pdu::PduParseError;

#[derive(Debug, PartialEq)]
pub struct SubmitSmPdu(SmData);

impl SubmitSmPdu {
    pub fn new(
        service_type: &str,
        source_addr_ton: u8,
        source_addr_npi: u8,
        source_addr: &str,
        dest_addr_ton: u8,
        dest_addr_npi: u8,
        destination_addr: &str,
        esm_class: u8,
        protocol_id: u8,
        priority_flag: u8,
        schedule_delivery_time: &str,
        validity_period: &str,
        registered_delivery: u8,
        replace_if_present_flag: u8,
        data_coding: u8,
        sm_default_msg_id: u8,
        short_message: &[u8],
        tlvs: Tlvs,
    ) -> Result<Self, PduParseError> {
        // Later: Issue#6: validate esm_class for the type of message this is?
        Ok(Self(SmData::new(
            service_type,
            source_addr_ton,
            source_addr_npi,
            source_addr,
            dest_addr_ton,
            dest_addr_npi,
            destination_addr,
            esm_class,
            protocol_id,
            priority_flag,
            schedule_delivery_time,
            validity_period,
            registered_delivery,
            replace_if_present_flag,
            data_coding,
            sm_default_msg_id,
            short_message,
            tlvs,
        )?))
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        self.0.write(stream).await
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
        command_status: u32,
    ) -> Result<SubmitSmPdu, PduParseError> {
        // Later: Issue#6: validate esm_class for the type of message this is?
        Ok(Self(SmData::parse(bytes, command_status)?))
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        Ok(Self(self.0.validate_command_status(command_status)?))
    }

    pub fn destination_addr(&self) -> String {
        self.0.destination_addr.value.to_string()
    }
}
