use std::io;

use crate::pdu::formats::{COctetString, WriteStream};
use crate::pdu::pduparseerror::fld;
use crate::pdu::{PduParseError, PduParseErrorBody};

const MAX_LENGTH_SYSTEM_ID: usize = 16;

#[derive(Debug, PartialEq)]
struct Body {
    pub system_id: COctetString,
}

impl Body {
    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        self.system_id.write(stream).await
    }
}

#[derive(Debug, PartialEq)]
pub struct BindRespData {
    body: Option<Body>,
}

impl BindRespData {
    pub fn new(system_id: &str) -> Result<Self, PduParseError> {
        Ok(Self {
            body: Some(Body {
                system_id: fld(
                    "system_id",
                    COctetString::from_str(system_id, MAX_LENGTH_SYSTEM_ID),
                )?,
            }),
        })
    }

    pub fn new_error() -> Self {
        Self { body: None }
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        if let Some(body) = &self.body {
            body.write(stream).await
        } else {
            Ok(())
        }
    }

    pub fn parse(
        bytes: &mut dyn io::BufRead,
        command_status: u32,
    ) -> Result<BindRespData, PduParseError> {
        let body = if command_status == 0x00000000 {
            Some(Body {
                system_id: fld(
                    "system_id",
                    COctetString::read(bytes, MAX_LENGTH_SYSTEM_ID),
                )?,
            })
        } else {
            None
        };

        Ok(Self { body })
    }

    pub fn validate_command_status(
        self,
        command_status: u32,
    ) -> Result<Self, PduParseError> {
        match (&self.body, command_status) {
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
