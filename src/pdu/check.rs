use std::convert::TryFrom;
use std::io;

use crate::pdu::formats::Integer4;
use crate::pdu::validate_command_length::validate_command_length;
use crate::pdu::CheckOutcome;
use crate::result::Result;

pub fn check(bytes: &mut dyn io::BufRead) -> Result<CheckOutcome> {
    let command_length = Integer4::read(bytes);
    match command_length {
        Ok(command_length) => {
            validate_command_length(&command_length)?;
            check_can_read(bytes, command_length.value)
        }
        Err(e) => match e.kind() {
            io::ErrorKind::UnexpectedEof => Ok(CheckOutcome::Incomplete),
            _ => Err(e.into()),
        },
    }
}

fn check_can_read(
    bytes: &mut dyn io::BufRead,
    command_length: u32,
) -> Result<CheckOutcome> {
    let len = usize::try_from(command_length - 4)?;
    // Is there a better way than allocating this vector?
    let mut buf = Vec::with_capacity(len);
    buf.resize(len, 0);
    bytes
        .read_exact(buf.as_mut_slice())
        .map(|_| CheckOutcome::Ok)
        .or_else(|e| match e.kind() {
            io::ErrorKind::UnexpectedEof => Ok(CheckOutcome::Incomplete),
            _ => Err(e.into()),
        })
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
        assert_eq!(check(&mut cursor).unwrap(), CheckOutcome::Ok);
    }

    #[test]
    fn check_is_ok_if_exact_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1b]);
        assert_eq!(check(&mut cursor).unwrap(), CheckOutcome::Ok);
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
        let res = check(&mut failing_read).unwrap_err();
        assert_eq!(res.to_string(), FailingRead::error_string());
    }

    #[test]
    fn check_errors_if_short_length() {
        const PDU: &[u8; 4] = b"\x00\x00\x00\x04";
        let mut cursor = Cursor::new(&PDU);

        let res = check(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "PDU too short!  Length: 4, min allowed: 8."
        );
    }

    #[test]
    fn check_errors_without_reading_all_if_long_length() {
        const PDU: &[u8; 16] =
            b"\xff\xff\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00";
        let mut cursor = Cursor::new(&PDU);

        let res = check(&mut cursor).unwrap_err();
        assert_eq!(
            res.to_string(),
            "PDU too long!  Length: 4294967295, max allowed: 70000."
        );
    }
}
