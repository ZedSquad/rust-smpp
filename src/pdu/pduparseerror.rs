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
    LengthLongerThanPdu(u32),
    LengthTooLong(u32),
    LengthTooShort(u32),
    NotEnoughBytes,
    OctetStringCreationError(OctetStringCreationError),
    OtherIoError(io::Error),
    StatusIsNotZero(u32),
    UnknownCommandId,
}

#[derive(Debug)]
pub struct PduParseError {
    pub command_id: Option<u32>,
    sequence_number: Option<u32>, // Issue#1: populate and use this
    field_name: Option<String>,
    body: PduParseErrorBody,
}

impl PduParseError {
    pub fn for_unknown_command_id(command_id: u32) -> Self {
        Self {
            command_id: Some(command_id),
            sequence_number: None,
            field_name: None,
            body: PduParseErrorBody::UnknownCommandId,
        }
    }

    pub fn for_lengthlongerthanpdu(
        command_id: u32,
        command_length: u32,
    ) -> Self {
        Self {
            command_id: Some(command_id),
            sequence_number: None,
            field_name: None,
            body: PduParseErrorBody::LengthLongerThanPdu(command_length),
        }
    }

    pub fn for_statusisnotzero(status: u32) -> Self {
        Self {
            command_id: None,
            sequence_number: None,
            field_name: None,
            body: PduParseErrorBody::StatusIsNotZero(status),
        }
    }

    pub fn for_lengthtoolong(length: u32) -> Self {
        Self {
            command_id: None,
            sequence_number: None,
            field_name: None,
            body: PduParseErrorBody::LengthTooLong(length),
        }
    }

    pub fn for_lengthtooshort(length: u32) -> Self {
        Self {
            command_id: None,
            sequence_number: None,
            field_name: None,
            body: PduParseErrorBody::LengthTooShort(length),
        }
    }

    pub fn for_ioerror(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::UnexpectedEof => Self {
                command_id: None,
                sequence_number: None,
                field_name: None,
                body: PduParseErrorBody::NotEnoughBytes,
            },

            _ => Self {
                command_id: None,
                sequence_number: None,
                field_name: None,
                body: PduParseErrorBody::OtherIoError(e),
            },
        }
    }

    pub fn into_with_command_id(mut self, command_id: u32) -> Self {
        self.command_id = Some(command_id);
        self
    }

    pub fn into_with_field_name(mut self, field_name: &str) -> Self {
        self.field_name = Some(String::from(field_name));
        self
    }
}

impl From<OctetStringCreationError> for PduParseError {
    fn from(e: OctetStringCreationError) -> Self {
        Self {
            command_id: None,
            sequence_number: None,
            field_name: None,
            body: PduParseErrorBody::OctetStringCreationError(e),
        }
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
                PduParseError::for_lengthtoolong(length)
            }
            CommandLengthError::TooShort(length) => {
                PduParseError::for_lengthtooshort(length)
            }
        }
    }
}

impl From<io::Error> for PduParseError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::UnexpectedEof => Self {
                command_id: None,
                sequence_number: None,
                field_name: None,
                body: PduParseErrorBody::NotEnoughBytes,
            },
            _ => Self {
                command_id: None,
                sequence_number: None,
                field_name: None,
                body: PduParseErrorBody::OtherIoError(e),
            },
        }
    }
}

fn as_hex(num: Option<u32>) -> String {
    num.map(|i| format!("{:#010X}", i))
        .unwrap_or(String::from("UNKNOWN"))
}

impl Display for PduParseError {
    fn fmt(
        &self,
        formatter: &mut Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        let msg = match &self.body {
            PduParseErrorBody::OtherIoError(e) => format!("IO error: {}", e.to_string()),
            PduParseErrorBody::LengthTooLong(length) => format!("Length ({}) too long.  Max allowed is {} octets.", length, MAX_PDU_LENGTH),
            PduParseErrorBody::LengthTooShort(length) => format!("Length ({}) too short.  Min allowed is {} octets.", length, MIN_PDU_LENGTH),
            PduParseErrorBody::NotEnoughBytes => String::from("Reached end of PDU length (or end of input) before finding all fields of the PDU."),
            PduParseErrorBody::StatusIsNotZero(status) => format!("command_status must be 0, but was {}.", status),
            PduParseErrorBody::UnknownCommandId => String::from("Supplied command_id is unknown."),
            PduParseErrorBody::OctetStringCreationError(e) => e.to_string(),
            PduParseErrorBody::LengthLongerThanPdu(length) => format!("Finished parsing PDU but its length ({}) suggested it was longer.", length)
        };

        formatter.write_fmt(format_args!(
            "Error parsing PDU (command_id={}, field_name={}): {}",
            // Issue#1: Should be: "Error parsing PDU (command_id={}, sequence_number={}, field_name={}): {}",
            as_hex(self.command_id),
            // Issue#1: Should be: as_hex(self.sequence_number),
            self.field_name.clone().unwrap_or(String::from("UNKNOWN")),
            msg,
        ))
    }
}

impl error::Error for PduParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatting_unknown_command_id() {
        assert_eq!(
            PduParseError::for_unknown_command_id(0x00001234).to_string(),
            "Error parsing PDU (command_id=0x00001234, field_name=UNKNOWN): \
            Supplied command_id is unknown."
        );
    }
}
