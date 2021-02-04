use std::convert::TryFrom;
use std::io;

use crate::pdu::formats::Integer4;
use crate::pdu::validate_command_length::{
    validate_command_length, CommandLengthError,
};

#[derive(Debug)]
pub enum CheckError {
    CommandLengthError(CommandLengthError),
    IoError(io::Error),
}

// TODO: use these, and more ? below.
/*impl From<CommandLengthError> for CheckError {
    fn from(e: CommandLengthError) -> Self {
        CheckError::CommandLengthError(e)
    }
}*/

#[derive(Debug, PartialEq)]
pub enum CheckOutcome {
    Ready,
    Incomplete,
}

pub fn check(bytes: &mut dyn io::BufRead) -> Result<CheckOutcome, CheckError> {
    Integer4::read(bytes)
        .map(|len| {
            match validate_command_length(&len) {
                Ok(()) => (),
                Err(e) => return Err(CheckError::CommandLengthError(e)),
            }
            check_can_read(bytes, len.value)
        })
        .unwrap_or_else(result_from_io_error)
}

fn result_from_io_error(
    io_error: io::Error,
) -> Result<CheckOutcome, CheckError> {
    match io_error.kind() {
        io::ErrorKind::UnexpectedEof => Ok(CheckOutcome::Incomplete),
        _ => Err(CheckError::IoError(io_error)),
    }
}

fn check_can_read(
    bytes: &mut dyn io::BufRead,
    command_length: u32,
) -> Result<CheckOutcome, CheckError> {
    let len = usize::try_from(command_length - 4).map_err(|_| {
        CheckError::CommandLengthError(CommandLengthError::TooShort(
            command_length,
        ))
    })?;
    // Is there a better way than allocating this vector?
    let mut buf = Vec::with_capacity(len);
    buf.resize(len, 0);
    bytes
        .read_exact(buf.as_mut_slice())
        .map(|_| CheckOutcome::Ready)
        .or_else(result_from_io_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::unittest_utils::FailingRead;

    const BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA: &[u8; 0x1b + 0xa] =
        b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02TestServer\0extrabytes";

    #[test]
    fn check_is_ok_if_more_bytes() {
        let mut cursor = Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..]);
        assert_eq!(check(&mut cursor).unwrap(), CheckOutcome::Ready);
    }

    #[test]
    fn check_is_ok_if_exact_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1b]);
        assert_eq!(check(&mut cursor).unwrap(), CheckOutcome::Ready);
    }

    #[test]
    fn check_is_incomplete_if_fewer_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1a]);
        assert_eq!(check(&mut cursor).unwrap(), CheckOutcome::Incomplete);
    }

    #[test]
    fn check_errors_if_read_error() {
        let mut failing_read = FailingRead::new_bufreader();
        assert!(matches!(
            check(&mut failing_read).unwrap_err(),
            CheckError::IoError(_)
        ));
    }

    #[test]
    fn check_errors_if_short_length() {
        const PDU: &[u8; 4] = b"\x00\x00\x00\x04";
        let mut cursor = Cursor::new(&PDU);

        let res = check(&mut cursor).unwrap_err();
        assert!(matches!(
            res,
            CheckError::CommandLengthError(CommandLengthError::TooShort(4))
        ));
    }

    #[test]
    fn check_errors_without_reading_all_if_long_length() {
        const PDU: &[u8; 16] =
            b"\xff\xff\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00";
        let mut cursor = Cursor::new(&PDU);

        let res = check(&mut cursor).unwrap_err();
        assert!(matches!(
            res,
            CheckError::CommandLengthError(CommandLengthError::TooLong(
                4294967295
            ))
        ));
    }
}
