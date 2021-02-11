use std::io;
use std::io::Read;

use crate::pdu::formats::{COctetString, WriteStream};
use crate::pdu::pduparseerror::fld;
use crate::pdu::{PduParseError, PduParseErrorBody};

// https://smpp.org/SMPP_v3_4_Issue1_2.pdf
// 4.4.2 lists both 9 and 33 crossed out, before listing 65 as the
// max size of the message_id.
const MAX_LENGTH_MESSAGE_ID: usize = 65;

#[derive(Debug, PartialEq)]
pub struct SubmitSmRespPdu {
    // If status != 0, message_id is None
    message_id: Option<COctetString>,
}

impl SubmitSmRespPdu {
    pub fn new(message_id: &str) -> Result<Self, PduParseError> {
        Ok(Self {
            message_id: Some(COctetString::from_str(
                message_id,
                MAX_LENGTH_MESSAGE_ID,
            )?),
        })
    }

    pub fn new_error() -> Self {
        Self { message_id: None }
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
        command_status: u32,
    ) -> Result<SubmitSmRespPdu, PduParseError> {
        if command_status == 0x00000000 {
            let message_id = Some(fld(
                "message_id",
                COctetString::read(bytes, MAX_LENGTH_MESSAGE_ID),
            )?);
            Ok(Self { message_id })
        } else {
            if let Some(_) = bytes.bytes().next() {
                return Err(PduParseError::new(
                    PduParseErrorBody::BodyNotAllowedWhenStatusIsNotZero,
                ));
            }

            Ok(Self { message_id: None })
        }
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        // TODO: This is identical to the code in BindTransmitterRespPdu
        match (&self.message_id, command_status) {
            (Some(_), 0) => Ok(self),
            (None, 0) => Err(PduParseError::new(
                PduParseErrorBody::BodyNotAllowedWhenStatusIsNotZero,
            )),
            (Some(_), _) => Err(PduParseError::new(
                PduParseErrorBody::BodyRequiredWhenStatusIsZero,
            )),
            (None, _) => Ok(self),
        }
    }
}
