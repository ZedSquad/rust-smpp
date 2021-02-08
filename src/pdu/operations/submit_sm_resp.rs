use std::io;
use std::io::Read;

use crate::pdu::formats::{COctetString, Integer4, WriteStream};
use crate::pdu::pduparseerror::fld;
use crate::pdu::PduParseError;

// https://smpp.org/SMPP_v3_4_Issue1_2.pdf
// 4.4.2 lists both 9 and 33 crossed out, before listing 65 as the
// max size of the message_id.
const MAX_LENGTH_MESSAGE_ID: usize = 65;

#[derive(Debug, PartialEq)]
pub struct SubmitSmRespPdu {
    command_status: Integer4,
    sequence_number: Integer4,
    message_id: COctetString,
    // message_id is Only non-empty if command_status == 0
    // We could use an enum to enforce this.
    // Currently we enforce via constructor only.
}

impl SubmitSmRespPdu {
    pub fn new_ok(
        sequence_number: u32,
        message_id: &str,
    ) -> Result<Self, PduParseError> {
        Ok(Self {
            command_status: Integer4::new(0),
            sequence_number: Integer4::new(sequence_number),
            message_id: COctetString::from_str(
                message_id,
                MAX_LENGTH_MESSAGE_ID,
            )?,
        })
    }

    pub fn new_error(
        command_status: u32,
        sequence_number: u32,
    ) -> Result<Self, PduParseError> {
        Ok(Self {
            command_status: Integer4::new(command_status),
            sequence_number: Integer4::new(sequence_number),
            message_id: COctetString::new(),
        })
    }

    pub async fn write(&self, _stream: &mut WriteStream) -> io::Result<()> {
        todo!()
    }

    /// Parse a submit_sm_resp PDU.
    /// Note: if command_status is non-zero, this function will attempt to
    /// read beyond the end of the PDU.  It does this to check whether
    /// a message_id has been supplied when it should not have been.
    /// This means that you must restrict the number of bytes available
    /// to read before entering this function.
    pub fn parse(
        bytes: &mut dyn io::BufRead,
    ) -> Result<SubmitSmRespPdu, PduParseError> {
        let command_status = fld("command_status", Integer4::read(bytes))?;
        let sequence_number = fld("sequence_number", Integer4::read(bytes))?;

        if command_status.value == 0 {
            let message_id = fld(
                "message_id",
                COctetString::read(bytes, MAX_LENGTH_MESSAGE_ID),
            )?;
            Ok(Self {
                command_status,
                sequence_number,
                message_id,
            })
        } else {
            if let Some(_) = bytes.bytes().next() {
                return Err(
                    PduParseError::for_bodynotallowedwhenstatusisnotzero(
                        command_status.value,
                    ),
                );
            }

            Ok(Self {
                command_status,
                sequence_number,
                message_id: COctetString::new(),
            })
        }
    }
}
