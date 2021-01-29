use core::fmt::Formatter;
use std::convert::TryFrom;
use std::error;
use std::fmt::Display;
use std::io;

use crate::pdu::formats::Integer4;
use crate::pdu::validate_command_length::validate_command_length;

#[derive(Debug, PartialEq)]
pub struct CheckError {
    pub message: String,
    pub io_errorkind: Option<io::ErrorKind>,
}

impl Display for CheckError {
    fn fmt(
        &self,
        formatter: &mut Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        if let Some(ek) = self.io_errorkind {
            formatter.write_fmt(format_args!(
                "Error checking PDU length: {}.  io::ErrorKind={:?}",
                self.message, ek
            ))
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl error::Error for CheckError {}

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
                Err(e) => {
                    return Err(CheckError {
                        message: e.message,
                        io_errorkind: None,
                    })
                }
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
        _ => Err(CheckError {
            message: io_error.to_string(),
            io_errorkind: Some(io_error.kind()),
        }),
    }
}

fn check_can_read(
    bytes: &mut dyn io::BufRead,
    command_length: u32,
) -> Result<CheckOutcome, CheckError> {
    let len = usize::try_from(command_length - 4).map_err(|_| CheckError {
        message: String::from("Invalid command length."),
        io_errorkind: None,
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
        assert_eq!(check(&mut cursor), Ok(CheckOutcome::Ready));
    }

    #[test]
    fn check_is_ok_if_exact_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1b]);
        assert_eq!(check(&mut cursor), Ok(CheckOutcome::Ready));
    }

    #[test]
    fn check_is_incomplete_if_fewer_bytes() {
        let mut cursor =
            Cursor::new(&BIND_TRANSMITTER_RESP_PDU_PLUS_EXTRA[..0x1a]);
        assert_eq!(check(&mut cursor), Ok(CheckOutcome::Incomplete));
    }

    #[test]
    fn check_errors_if_read_error() {
        let mut failing_read = FailingRead::new_bufreader();
        assert_eq!(
            check(&mut failing_read),
            Err(CheckError {
                message: String::from("Invalid argument (os error 22)"),
                io_errorkind: Some(io::ErrorKind::InvalidInput)
            })
        );
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
