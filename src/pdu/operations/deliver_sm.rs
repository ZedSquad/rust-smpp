use std::io;
use std::str::from_utf8;

use crate::pdu::data::sm_data::SmData;
use crate::pdu::formats::WriteStream;
use crate::pdu::PduParseError;

#[derive(Debug, PartialEq)]
pub struct DeliverSmPdu(SmData);

impl DeliverSmPdu {
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
        )?))
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        self.0.write(stream).await
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
        command_status: u32,
    ) -> Result<DeliverSmPdu, PduParseError> {
        // Later: Issue#6: validate esm_class for the type of message this is?
        Ok(Self(SmData::parse(bytes, command_status)?))
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        Ok(Self(self.0.validate_command_status(command_status)?))
    }

    pub fn extract_receipted_message_id(&self) -> Option<String> {
        if self.0.short_message.value.starts_with(b"id:") {
            // Later: Issue#7: assumes the whole short message is just id
            from_utf8(&self.0.short_message.value[3..])
                .ok()
                .map(String::from)
        } else {
            None
        }
    }

    pub fn source_addr(&self) -> String {
        self.0.source_addr.value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_id_is_at_start_of_short_message_and_no_tlv_we_can_extract_id() {
        let deliver_sm = DeliverSmPdu::new(
            "",
            0,
            0,
            "",
            0,
            0,
            "",
            0,
            0,
            0,
            "",
            "",
            0,
            0,
            0,
            0,
            b"id:0123456789",
        )
        .unwrap();
        assert_eq!(
            deliver_sm.extract_receipted_message_id().unwrap(),
            "0123456789"
        );
    }
}

// Later: Issue#2: Extract message id from receipted_message_id TLV
// Later: Issue#7: parse short_message more fully - e.g. id not at start
// Later: Issue#17: Explicitly allow/disallow short_message ids longer than 10?
// Later: Issue#17: Explicitly allow/disallow short_message ids not decimal?
// Later: Issue#17: https://smpp.org/SMPP_v3_4_Issue1_2.pdf Appendix B says ID
//       is NULL-terminated ("C-Octet String (Decimal)"), but that
//       seems unlikely - check real-world usage.
