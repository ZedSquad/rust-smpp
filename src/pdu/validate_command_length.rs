use crate::pdu::formats::Integer4;

// https://smpp.org/smppv34_gsmumts_ig_v10.pdf p11 states:
// "... message_payload parameter which can hold up to a maximum of 64K ..."
// So we guess no valid PDU can be longer than 70K octets.
const MAX_PDU_LENGTH: usize = 70000;

// We need at least a command_length and command_id, so 8 bytes
const MIN_PDU_LENGTH: usize = 8;

pub fn validate_command_length(
    command_length: &Integer4,
) -> Result<(), String> {
    let len = command_length.value as usize;
    if len > MAX_PDU_LENGTH {
        Err(format!(
            "PDU too long!  Length: {}, max allowed: {}.",
            len, MAX_PDU_LENGTH
        ))
    } else if len < MIN_PDU_LENGTH {
        Err(format!(
            "PDU too short!  Length: {}, min allowed: {}.",
            len, MIN_PDU_LENGTH
        ))
    } else {
        Ok(())
    }
}
