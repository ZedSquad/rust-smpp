use crate::pdu::formats::Integer4;

// https://smpp.org/smppv34_gsmumts_ig_v10.pdf p11 states:
// "... message_payload parameter which can hold up to a maximum of 64K ..."
// So we guess no valid PDU can be longer than 70K octets.
pub const MAX_PDU_LENGTH: usize = 70000;

// We need at least a command_length and command_id, so 8 bytes
pub const MIN_PDU_LENGTH: usize = 8;

#[derive(Debug)]
pub enum CommandLengthError {
    TooLong(u32),
    TooShort(u32),
}

pub fn validate_command_length(
    command_length: &Integer4,
) -> Result<(), CommandLengthError> {
    let len = command_length.value;
    if len > MAX_PDU_LENGTH as u32 {
        Err(CommandLengthError::TooLong(len))
    } else if len < MIN_PDU_LENGTH as u32 {
        Err(CommandLengthError::TooShort(len))
    } else {
        Ok(())
    }
}
