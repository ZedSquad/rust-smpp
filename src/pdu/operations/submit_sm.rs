use std::io;

use crate::pdu::formats::{
    COctetString, Integer1, Integer4, OctetString, WriteStream,
};
use crate::pdu::pduparseerror::fld;
use crate::pdu::PduParseError;

const MAX_LENGTH_SERVICE_TYPE: usize = 6;
const MAX_LENGTH_SOURCE_ADDR: usize = 21;
const MAX_LENGTH_DESTINATION_ADDR: usize = 21;
const MAX_LENGTH_SCHEDULE_DELIVERY_TIME: usize = 17;
const MAX_LENGTH_VALIDITY_PERIOD: usize = 17;
const MAX_LENGTH_SHORT_MESSAGE: usize = 254;

#[derive(Debug, PartialEq)]
pub struct SubmitSmPdu {
    pub sequence_number: Integer4,
    service_type: COctetString,
    source_addr_ton: Integer1,
    source_addr_npi: Integer1,
    source_addr: COctetString,
    dest_addr_ton: Integer1,
    dest_addr_npi: Integer1,
    destination_addr: COctetString,
    esm_class: Integer1,
    protocol_id: Integer1,
    priority_flag: Integer1,
    schedule_delivery_time: COctetString,
    validity_period: COctetString,
    registered_delivery: Integer1,
    replace_if_present_flag: Integer1,
    data_coding: Integer1,
    sm_default_msg_id: Integer1,
    short_message: OctetString,
    // TODO: TLVs
}

fn validate_length_1_or_17(
    field_name: &str,
    length: usize,
) -> Result<(), PduParseError> {
    // We have already removed the trailing NULL character, so we actually
    // check for length 0 or 16.
    if length == 0 || length == 16 {
        Ok(())
    } else {
        Err(PduParseError::for_incorrect_length(
            length as u32,
            "Must be either 1 or 17 characters, including the NULL character.",
        )
        .into_with_field_name(field_name))
    }
}

impl SubmitSmPdu {
    pub fn new(
        sequence_number: u32,
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
        validate_length_1_or_17(
            "schedule_delivery_time",
            schedule_delivery_time.len(),
        )?;
        validate_length_1_or_17("validity_period", validity_period.len())?;

        Ok(Self {
            sequence_number: Integer4::new(sequence_number),
            service_type: COctetString::from_str(
                service_type,
                MAX_LENGTH_SERVICE_TYPE,
            )?,
            source_addr_ton: Integer1::new(source_addr_ton),
            source_addr_npi: Integer1::new(source_addr_npi),
            source_addr: COctetString::from_str(
                source_addr,
                MAX_LENGTH_SOURCE_ADDR,
            )?,
            dest_addr_ton: Integer1::new(dest_addr_ton),
            dest_addr_npi: Integer1::new(dest_addr_npi),
            destination_addr: COctetString::from_str(
                destination_addr,
                MAX_LENGTH_DESTINATION_ADDR,
            )?,
            esm_class: Integer1::new(esm_class),
            protocol_id: Integer1::new(protocol_id),
            priority_flag: Integer1::new(priority_flag),
            schedule_delivery_time: COctetString::from_str(
                schedule_delivery_time,
                MAX_LENGTH_SCHEDULE_DELIVERY_TIME,
            )?,
            validity_period: fld(
                "validity_period",
                COctetString::from_str(
                    validity_period,
                    MAX_LENGTH_VALIDITY_PERIOD,
                ),
            )?,
            registered_delivery: Integer1::new(registered_delivery),
            replace_if_present_flag: Integer1::new(replace_if_present_flag),
            data_coding: Integer1::new(data_coding),
            sm_default_msg_id: Integer1::new(sm_default_msg_id),
            short_message: fld(
                "short_message",
                OctetString::from_bytes(
                    short_message,
                    MAX_LENGTH_SHORT_MESSAGE,
                ),
            )?,
        })
    }

    pub async fn write(&self, _stream: &mut WriteStream) -> io::Result<()> {
        todo!()
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
    ) -> Result<SubmitSmPdu, PduParseError> {
        let command_status = fld("command_status", Integer4::read(bytes))?;
        let sequence_number = fld("sequence_number", Integer4::read(bytes))?;
        let service_type = fld(
            "service_type",
            COctetString::read(bytes, MAX_LENGTH_SERVICE_TYPE),
        )?;
        let source_addr_ton = fld("source_addr_ton", Integer1::read(bytes))?;
        let source_addr_npi = fld("source_addr_npi", Integer1::read(bytes))?;
        let source_addr = fld(
            "source_addr",
            COctetString::read(bytes, MAX_LENGTH_SOURCE_ADDR),
        )?;
        let dest_addr_ton = fld("dest_addr_ton", Integer1::read(bytes))?;
        let dest_addr_npi = fld("dest_addr_npi", Integer1::read(bytes))?;
        let destination_addr = fld(
            "destination_addr",
            COctetString::read(bytes, MAX_LENGTH_DESTINATION_ADDR),
        )?;
        let esm_class = fld("esm_class", Integer1::read(bytes))?;
        let protocol_id = fld("protocol_id", Integer1::read(bytes))?;
        let priority_flag = fld("priority_flag", Integer1::read(bytes))?;
        let schedule_delivery_time = fld(
            "schedule_delivery_time",
            COctetString::read(bytes, MAX_LENGTH_SCHEDULE_DELIVERY_TIME),
        )?;
        let validity_period = fld(
            "validity_period",
            COctetString::read(bytes, MAX_LENGTH_VALIDITY_PERIOD),
        )?;
        let registered_delivery =
            fld("registered_delivery", Integer1::read(bytes))?;
        let replace_if_present_flag =
            fld("replace_if_present_flag", Integer1::read(bytes))?;
        let data_coding = fld("data_coding", Integer1::read(bytes))?;
        let sm_default_msg_id =
            fld("sm_default_msg_id", Integer1::read(bytes))?;
        let sm_length = fld("sm_length", Integer1::read(bytes))?;
        let short_message = fld(
            "short_message",
            OctetString::read(
                bytes,
                sm_length.value as usize,
                MAX_LENGTH_SHORT_MESSAGE,
            ),
        )?;

        if command_status.value != 0x00 {
            return Err(PduParseError::for_statusisnotzero(
                command_status.value,
            )
            .into_with_field_name("command_status"));
        }

        validate_length_1_or_17(
            "schedule_delivery_time",
            schedule_delivery_time.value.len(),
        )?;
        validate_length_1_or_17(
            "validity_period",
            validity_period.value.len(),
        )?;
        // TODO: check EITHER short_message, or message_payload TLV

        Ok(Self {
            sequence_number,
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
        })
    }
}
