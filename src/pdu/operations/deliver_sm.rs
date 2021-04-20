use regex::Regex;
use std::io;
use std::str;

use crate::pdu::data::sm_data::SmData;
use crate::pdu::formats::WriteStream;
use crate::pdu::tlvs::{KnownTlvTag, Tlvs};
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
        if let Some(tlv) = self.0.tlvs.get(KnownTlvTag::receipted_message_id) {
            return String::from_utf8(tlv.value).ok().map(|mut s| {
                if s.ends_with('\0') {
                    s.truncate(s.len() - 1)
                }
                s
            });
        }

        lazy_static! {
            static ref RE: Regex = Regex::new(r"(?i)\bid:(\S*)(\s|$)").unwrap();
        }

        str::from_utf8(&self.0.short_message.value)
            .ok()
            .and_then(|sm| {
                RE.captures(sm)
                    .map(|caps| String::from(caps.get(1).unwrap().as_str()))
            })
    }

    pub fn source_addr(&self) -> String {
        self.0.source_addr.value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdu::tlvs::{KnownTlvTag, Tlv};

    #[test]
    fn when_id_is_at_start_of_short_message_and_no_tlv_we_can_extract_id() {
        assert_eq!(
            dr("id:0123456789").extract_receipted_message_id().unwrap(),
            "0123456789"
        );
    }

    #[test]
    fn can_parse_short_message_from_appendix_b() {
        let msg = dr("id:2345678901   sub:001   dlvrd:001   \
             submit   date:2103291550   donedate:2103291551 \
             stat:DELIVRD err:AOK \
             Text:012345678 abcdefghij");

        assert_eq!(msg.extract_receipted_message_id().unwrap(), "2345678901");
    }

    #[test]
    fn can_parse_more_realistic_dr_short_messages() {
        assert_eq!(
            dr("id:2236192998 sub:001 dlvrd:001 submit date:1606160544 \
                done date:1606160544 stat:DELIVRD \
                err:000 text:\u{0006}\u{0005}\u{0004}\u{0015}y\000\000\
                FOO-BARR?v=3;")
            .extract_receipted_message_id()
            .unwrap(),
            "2236192998"
        );
        assert_eq!(
            dr(
                "iD:1A2a71714437 submt daTE:1606161059 doNE date:1606161059 \
               sTat:DELIvRD eRr:0a0z0"
            )
            .extract_receipted_message_id()
            .unwrap(),
            "1A2a71714437"
        );
        assert_eq!(
            dr("iD:1A:-/2a71714437 submt daTE:1606161059 \
                doNE date:1606161059 sTat:DELIvRD eRr:0a0z0")
            .extract_receipted_message_id()
            .unwrap(),
            "1A:-/2a71714437"
        );
        assert_eq!(
            dr("submit date:1404161533 id::1-/2345:67890: sub:001 \
                dlvrd:001 done date:1404161535 stat:DELIVRD")
            .extract_receipted_message_id()
            .unwrap(),
            ":1-/2345:67890:"
        );
        assert_eq!(
            dr("id:632166473 sub:001 dlvrd:001 submit date:1907101637 \
                done date:1907101637 stat:UNDELIV err:MI:0024, text:")
            .extract_receipted_message_id()
            .unwrap(),
            "632166473"
        );
        assert_eq!(
            dr("submit date:1404161533 id:123456 sub:001 dlvrd:001 \
                done date:1404161535 stat:DELIVRD")
            .extract_receipted_message_id()
            .unwrap(),
            "123456"
        );
    }

    #[test]
    fn when_id_is_in_tlv_we_can_extract_it() {
        assert_eq!(
            dr_tlvs(
                "",
                Tlvs::from(&[Tlv::new(
                    KnownTlvTag::receipted_message_id,
                    "01234567890123456789".as_bytes()
                )])
            )
            .extract_receipted_message_id()
            .unwrap(),
            "01234567890123456789"
        );
    }

    #[test]
    fn when_null_terminated_id_is_in_tlv_we_can_extract_it() {
        assert_eq!(
            dr_tlvs(
                "",
                Tlvs::from(&[Tlv::new(
                    KnownTlvTag::receipted_message_id,
                    "01234567890123456789\0".as_bytes()
                )])
            )
            .extract_receipted_message_id()
            .unwrap(),
            "01234567890123456789"
        );
    }

    #[test]
    fn when_id_is_in_tlv_and_short_message_tlv_wins() {
        assert_eq!(
            dr_tlvs(
                "id:abc",
                Tlvs::from(&[Tlv::new(
                    KnownTlvTag::receipted_message_id,
                    "01234567890123456789".as_bytes()
                )])
            )
            .extract_receipted_message_id()
            .unwrap(),
            "01234567890123456789"
        );
    }

    fn dr(short_message: &str) -> DeliverSmPdu {
        let sm = short_message.as_bytes();
        let tlvs = Tlvs::new();
        DeliverSmPdu::new(
            "", 0, 0, "", 0, 0, "", 0, 0, 0, "", "", 0, 0, 0, 0, sm, tlvs,
        )
        .unwrap()
    }

    fn dr_tlvs(short_message: &str, tlvs: Tlvs) -> DeliverSmPdu {
        let sm = short_message.as_bytes();
        DeliverSmPdu::new(
            "", 0, 0, "", 0, 0, "", 0, 0, 0, "", "", 0, 0, 0, 0, sm, tlvs,
        )
        .unwrap()
    }
}

// Later: Issue#17: Explicitly allow/disallow short_message ids longer than 10?
// Later: Issue#17: Explicitly allow/disallow short_message ids not decimal?
// Later: Issue#17: https://smpp.org/SMPP_v3_4_Issue1_2.pdf Appendix B says ID
//       is NULL-terminated ("C-Octet String (Decimal)"), but that
//       seems unlikely - check real-world usage.
// Later: Issue#18: Parse message id from message_content TLV
