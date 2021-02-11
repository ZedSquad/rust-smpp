use core::fmt::{Display, Formatter};
use std::error;
use std::io;

use crate::pdu::{
    CommandLengthError, OctetStringCreationError, MAX_PDU_LENGTH,
    MIN_PDU_LENGTH,
};

use super::CheckError;

#[derive(Debug)]
pub enum PduParseErrorBody {
    BodyNotAllowedWhenStatusIsNotZero,
    BodyRequiredWhenStatusIsZero,
    LengthLongerThanPdu(u32),
    LengthTooLong(u32),
    LengthTooShort(u32),
    IncorrectLength(u32, String),
    NotEnoughBytes,
    OctetStringCreationError(OctetStringCreationError),
    OtherIoError(io::Error),
    StatusIsNotZero,
    StatusIsZero,
    UnknownCommandId,
}

#[derive(Debug)]
pub struct PduParseError {
    pub command_id: Option<u32>,
    command_status: Option<u32>,
    pub sequence_number: Option<u32>,
    field_name: Option<String>,
    body: PduParseErrorBody,
}

impl PduParseError {
    pub fn new(body: PduParseErrorBody) -> Self {
        Self {
            command_id: None,
            command_status: None,
            sequence_number: None,
            field_name: None,
            body,
        }
    }

    pub fn into_with_header(
        mut self,
        command_id: Option<u32>,
        command_status: Option<u32>,
        sequence_number: Option<u32>,
    ) -> Self {
        self.command_id = command_id;
        self.command_status = command_status;
        self.sequence_number = sequence_number;
        self
    }

    pub fn into_with_field_name(mut self, field_name: &str) -> Self {
        self.field_name = Some(String::from(field_name));
        self
    }
}

impl From<OctetStringCreationError> for PduParseError {
    fn from(e: OctetStringCreationError) -> Self {
        Self::new(PduParseErrorBody::OctetStringCreationError(e))
    }
}

impl From<CheckError> for PduParseError {
    fn from(e: CheckError) -> Self {
        match e {
            CheckError::IoError(e) => e.into(),
            CheckError::CommandLengthError(e) => e.into(),
        }
    }
}

impl From<CommandLengthError> for PduParseError {
    fn from(e: CommandLengthError) -> Self {
        match e {
            CommandLengthError::TooLong(length) => {
                Self::new(PduParseErrorBody::LengthTooLong(length))
            }
            CommandLengthError::TooShort(length) => {
                Self::new(PduParseErrorBody::LengthTooShort(length))
            }
        }
    }
}

impl From<io::Error> for PduParseError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::UnexpectedEof => {
                Self::new(PduParseErrorBody::NotEnoughBytes)
            }
            _ => Self::new(PduParseErrorBody::OtherIoError(e)),
        }
    }
}

impl Display for PduParseError {
    fn fmt(
        &self,
        formatter: &mut Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        let msg = match &self.body {
            PduParseErrorBody::BodyNotAllowedWhenStatusIsNotZero => {
                format!(
                    "PDU body must not be supplied when status is not zero, \
                    but command_status is {}.",
                    as_hex(self.command_status)
                )
            }
            PduParseErrorBody::BodyRequiredWhenStatusIsZero => String::from(
                "PDU body must be supplied when status is zero, \
                    but it is missing.",
            ),
            PduParseErrorBody::IncorrectLength(length, message) => {
                format!("Length {} was incorrect: {}", length, message)
            }
            PduParseErrorBody::LengthLongerThanPdu(length) => format!(
                "Finished parsing PDU but its length ({}) suggested \
                    it was longer.",
                length
            ),
            PduParseErrorBody::LengthTooLong(length) => format!(
                "Length ({}) too long.  Max allowed is {} octets.",
                length, MAX_PDU_LENGTH
            ),
            PduParseErrorBody::LengthTooShort(length) => format!(
                "Length ({}) too short.  Min allowed is {} octets.",
                length, MIN_PDU_LENGTH
            ),
            PduParseErrorBody::NotEnoughBytes => String::from(
                "Reached end of PDU length (or end of input) before \
                    finding all fields of the PDU.",
            ),
            PduParseErrorBody::OctetStringCreationError(e) => e.to_string(),
            PduParseErrorBody::OtherIoError(e) => {
                format!("IO error: {}", e.to_string())
            }
            PduParseErrorBody::StatusIsNotZero => {
                format!(
                    "command_status must be 0, but was {}.",
                    as_hex(self.command_status)
                )
            }
            PduParseErrorBody::StatusIsZero => {
                String::from("command_status must not be non-zero, but was 0.")
            }
            PduParseErrorBody::UnknownCommandId => {
                String::from("Supplied command_id is unknown.")
            }
        };

        formatter.write_fmt(format_args!(
            "Error parsing PDU (\
            command_id={}, command_status={}, \
            sequence_number={}, field_name={}): {}",
            as_hex(self.command_id),
            as_hex(self.command_status),
            as_hex(self.sequence_number),
            self.field_name.clone().unwrap_or(String::from("UNKNOWN")),
            msg,
        ))
    }
}

impl error::Error for PduParseError {}

fn as_hex(num: Option<u32>) -> String {
    if let Some(num) = num {
        format!("{:#010X}", num)
    } else {
        String::from("UNKNOWN")
    }
}

/// If the supplied result is an error, enrich it with the supplied field name
pub fn fld<T, E>(
    field_name: &str,
    res: Result<T, E>,
) -> Result<T, PduParseError>
where
    E: Into<PduParseError>,
{
    res.map_err(|e| e.into().into_with_field_name(field_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatting_unknown_command_id() {
        assert_eq!(
            PduParseError::new(PduParseErrorBody::UnknownCommandId)
                .into_with_header(
                    Some(0x00001234),
                    Some(0x00000000),
                    Some(0x0004321)
                )
                .to_string(),
            "Error parsing PDU (\
            command_id=0x00001234, command_status=0x00000000, \
            sequence_number=0x00004321, field_name=UNKNOWN): \
            Supplied command_id is unknown."
        );
    }
}
