use ascii::{AsAsciiStrError, AsciiString, FromAsciiError};
use core::fmt::{Display, Formatter};
use std::error;
use std::io;
use std::io::{BufRead, Read};
use tokio::io::{AsyncWrite, AsyncWriteExt};

// TODO: PDU Types, from spec section 3.1
// COctetStringDecimal
// COctetStringHex
// OctetString

pub type WriteStream = dyn AsyncWrite + Send + Unpin;

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 3.1
///
/// Integer: (1 byte)
/// An unsigned value with the defined number of octets.
/// The octets will always be transmitted MSB first (Big Endian).
#[derive(Clone, Debug, PartialEq)]
pub struct Integer1 {
    pub value: u8,
}

impl Integer1 {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn read(bytes: &mut dyn BufRead) -> io::Result<Self> {
        // Is allocating this buffer the right way?
        let mut ret: [u8; 1] = [0; 1];
        bytes.read_exact(&mut ret)?;
        Ok(Self { value: ret[0] })
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        stream.write_u8(self.value).await
    }
}

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 3.1
///
/// Integer: (4 bytes)
/// An unsigned value with the defined number of octets.
/// The octets will always be transmitted MSB first (Big Endian).
#[derive(Clone, Debug, PartialEq)]
pub struct Integer4 {
    pub value: u32,
}

impl Integer4 {
    pub fn new(value: u32) -> Self {
        Self { value }
    }

    pub fn read(bytes: &mut dyn BufRead) -> io::Result<Self> {
        let mut ret: [u8; 4] = [0; 4];
        bytes.read_exact(&mut ret)?;
        Ok(Self {
            value: u32::from_be_bytes(ret),
        })
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        stream.write_u32(self.value).await
    }
}

#[derive(Debug)]
pub enum OctetStringCreationError {
    DoesNotEndWithZeroByte,
    NotAscii(AsAsciiStrError),
    TooLong(usize),
    OtherIoError(io::Error),
}

impl Display for OctetStringCreationError {
    fn fmt(
        &self,
        formatter: &mut Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        let s = match self {
            OctetStringCreationError::DoesNotEndWithZeroByte => String::from(
                "C-Octet String does not end with the NULL character.",
            ),
            OctetStringCreationError::NotAscii(e) => format!(
                "Octet String is not ASCII (valid up to byte {}).",
                e.valid_up_to()
            ),
            OctetStringCreationError::TooLong(max_len) => {
                format!(
                    "Octet String is too long.  \
                    Max length is {}, including final zero byte.",
                    max_len
                )
            }
            OctetStringCreationError::OtherIoError(e) => {
                format!("IO error creating Octet String: {}", e.to_string())
            }
        };
        formatter.write_str(&s)
    }
}

impl error::Error for OctetStringCreationError {}

impl From<io::Error> for OctetStringCreationError {
    fn from(e: io::Error) -> Self {
        OctetStringCreationError::OtherIoError(e)
    }
}

impl From<AsAsciiStrError> for OctetStringCreationError {
    fn from(e: AsAsciiStrError) -> Self {
        OctetStringCreationError::NotAscii(e)
    }
}

impl<Orig> From<FromAsciiError<Orig>> for OctetStringCreationError {
    fn from(e: FromAsciiError<Orig>) -> Self {
        OctetStringCreationError::NotAscii(e.ascii_error())
    }
}

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 3.1
///
/// C-Octet String:
/// A series of ASCII characters terminated with the NULL character.
#[derive(Clone, Debug, PartialEq)]
pub struct COctetString {
    pub value: AsciiString,
}

// To consider in future: types for e.g. system_id that are a COctetString
// with a fixed, known length.  Currently we check it on creation, but
// then forget it.  If the number of these things is small, it would be nice
// to know for sure we had the right length later, e.g. when we are writing
// it.

impl COctetString {
    pub fn from_bytes(
        value: &[u8],
        max_len: usize,
    ) -> Result<Self, OctetStringCreationError> {
        if value.len() < max_len {
            Ok(Self {
                value: AsciiString::from_ascii(value)?,
            })
        } else {
            Err(OctetStringCreationError::TooLong(max_len))
        }
    }
    pub fn from_str(
        value: &str,
        max_len: usize,
    ) -> Result<Self, OctetStringCreationError> {
        Self::from_bytes(value.as_bytes(), max_len)
    }

    pub fn read(
        bytes: &mut dyn BufRead,
        max_len: usize,
    ) -> Result<Self, OctetStringCreationError> {
        let mut buf = Vec::new();
        let num = bytes.take(max_len as u64).read_until(0x00, &mut buf)?;

        if buf.last() != Some(&0x00) {
            // Failed to read a NULL terminator before we ran out of characters
            if num == max_len {
                return Err(OctetStringCreationError::TooLong(max_len));
            } else {
                return Err(OctetStringCreationError::DoesNotEndWithZeroByte);
            }
        }

        let buf = &buf[..(buf.len() - 1)]; // Remove trailing 0 byte

        COctetString::from_bytes(buf, max_len)
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        stream.write_all(self.value.as_bytes()).await?;
        stream.write_u8(0u8).await
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }
}

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 3.1
///
/// Octet String:
/// A series of octets, not necessarily NULL terminated.
#[derive(Clone, Debug, PartialEq)]
pub struct OctetString {
    pub value: Vec<u8>,
}

impl OctetString {
    pub fn new(
        value: Vec<u8>,
        max_len: usize,
    ) -> Result<Self, OctetStringCreationError> {
        if value.len() < max_len {
            Ok(Self { value })
        } else {
            Err(OctetStringCreationError::TooLong(max_len))
        }
    }

    pub fn from_bytes(
        value: &[u8],
        max_len: usize,
    ) -> Result<Self, OctetStringCreationError> {
        if value.len() < max_len {
            let mut v = Vec::with_capacity(value.len());
            v.extend_from_slice(value);
            Ok(Self { value: v })
        } else {
            Err(OctetStringCreationError::TooLong(max_len))
        }
    }

    pub fn read(
        bytes: &mut dyn BufRead,
        length: usize,
        max_len: usize,
    ) -> Result<Self, OctetStringCreationError> {
        if length > max_len {
            return Err(OctetStringCreationError::TooLong(length));
        }

        let mut buf = Vec::with_capacity(length);
        buf.resize(length, 0x00);
        bytes.read_exact(buf.as_mut_slice())?;
        OctetString::new(buf, max_len)
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        stream.write_all(self.value.as_slice()).await
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unittest_utils::FailingRead;

    #[test]
    fn read_integer1() {
        let mut bytes = io::BufReader::new(&[0x23][..]);
        assert_eq!(Integer1::read(&mut bytes).unwrap(), Integer1::new(0x23));
    }

    #[test]
    fn read_error_integer1() {
        let mut failing_read = FailingRead::new_bufreader();
        let res = Integer1::read(&mut failing_read).unwrap_err();
        assert_eq!(res.to_string(), FailingRead::error_string());
    }

    #[tokio::test]
    async fn write_integer1() {
        let mut buf: Vec<u8> = Vec::new();
        Integer1::new(0xfe).write(&mut buf).await.unwrap();
        assert_eq!(buf, vec![0xfe]);
    }

    #[test]
    fn read_integer4() {
        let mut bytes = io::BufReader::new(&[0xf0, 0x00, 0x00, 0x23][..]);
        assert_eq!(
            Integer4::read(&mut bytes).unwrap(),
            Integer4::new(0xf0000023)
        );
    }

    #[test]
    fn read_error_integer4() {
        let mut failing_read = FailingRead::new_bufreader();
        let res = Integer4::read(&mut failing_read).unwrap_err();
        assert_eq!(res.to_string(), FailingRead::error_string());
    }

    #[tokio::test]
    async fn write_integer4() {
        let mut buf: Vec<u8> = Vec::new();
        Integer4::new(0x101010fe).write(&mut buf).await.unwrap();
        assert_eq!(buf, vec![0x10, 0x10, 0x10, 0xfe]);
    }

    #[test]
    fn read_coctetstring() {
        let mut bytes = io::BufReader::new("foobar\0".as_bytes());
        assert_eq!(
            COctetString::read(&mut bytes, 20).unwrap(),
            COctetString::from_str("foobar", 20).unwrap()
        );
    }

    #[test]
    fn read_coctetstring_max_length() {
        let mut bytes = io::BufReader::new("thisislong\0".as_bytes());
        assert_eq!(
            COctetString::read(&mut bytes, 11).unwrap(),
            COctetString::from_str("thisislong", 11).unwrap()
        );
    }

    #[test]
    fn read_error_coctetstring() {
        let mut failing_read = FailingRead::new_bufreader();
        let res = COctetString::read(&mut failing_read, 20).unwrap_err();
        assert!(matches!(res, OctetStringCreationError::OtherIoError(_)));
    }

    #[test]
    fn read_coctetstring_missing_zero_byte() {
        let mut bytes = io::BufReader::new("foobar".as_bytes());
        let res = COctetString::read(&mut bytes, 20).unwrap_err();
        assert!(matches!(
            res,
            OctetStringCreationError::DoesNotEndWithZeroByte
        ));
    }

    #[test]
    fn read_coctetstring_too_long() {
        let mut bytes = io::BufReader::new("foobar\0".as_bytes());
        let res = COctetString::read(&mut bytes, 3).unwrap_err();
        assert!(matches!(res, OctetStringCreationError::TooLong(3)));
    }

    #[test]
    fn read_coctetstring_zero_not_included_in_length() {
        let mut bytes = io::BufReader::new("foobar\0".as_bytes());
        let res = COctetString::read(&mut bytes, 6).unwrap_err();
        assert!(matches!(res, OctetStringCreationError::TooLong(6)));
    }

    #[tokio::test]
    async fn write_coctetstring() {
        let mut buf: Vec<u8> = Vec::new();
        let val = COctetString::from_str("abc", 16).unwrap();
        val.write(&mut buf).await.unwrap();
        assert_eq!(buf, vec!['a' as u8, 'b' as u8, 'c' as u8, 0x00]);
    }

    #[test]
    fn read_octetstring() {
        let mut bytes = io::BufReader::new("foobar\0".as_bytes());
        assert_eq!(
            OctetString::read(&mut bytes, 7, 20).unwrap(),
            OctetString::from_bytes(b"foobar\0", 20).unwrap()
        );
    }

    // TODO: more tests for OctetString, including write
}
