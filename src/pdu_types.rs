use ascii::{AsciiStr, AsciiString};
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
#[derive(Debug, PartialEq)]
pub struct Integer1 {
    pub value: u8,
}

impl Integer1 {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn read(bytes: &mut dyn Read) -> io::Result<Self> {
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
#[derive(Debug, PartialEq)]
pub struct Integer4 {
    pub value: u32,
}

impl Integer4 {
    pub fn new(value: u32) -> Self {
        Self { value }
    }

    pub fn read(bytes: &mut dyn Read) -> io::Result<Self> {
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

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 3.1
///
/// C-Octet String:
/// A series of ASCII characters terminated with the NULL character.
#[derive(Debug, PartialEq)]
pub struct COctetString {
    value: AsciiString,
}

// To consider in future: types for e.g. system_id that are a COctetString
// with a fixed, known length.  Currently we check it on creation, but
// then forget it.  If the number of these things is small, it would be nice
// to know for sure we had the right length later, e.g. when we are writing
// it.

impl COctetString {
    pub fn new(value: &AsciiStr, max_len: usize) -> Self {
        assert!(value.len() <= max_len);
        Self {
            value: AsciiString::from(value),
        }
    }

    pub fn read(
        bytes: &mut dyn BufRead,
        max_len: usize,
        field_name: &str,
    ) -> io::Result<Self> {
        let mut buf = Vec::new();
        bytes.take(max_len as u64).read_until(0x00, &mut buf)?;

        if buf.last() != Some(&0x00) {
            // Failed to read a NULL terminator before we ran out of characters
            return Err(inv(format!(
                "String value for {} is too long.  Max length is {}.",
                field_name, max_len
            )));
        }

        let buf = &buf[..(buf.len() - 1)]; // Remove trailing 0 byte
        AsciiStr::from_ascii(buf)
            .map(|s| COctetString::new(s, max_len))
            .map_err(|e| {
                inv(format!(
                    "String value of {} is not ASCII (valid up to byte {}).",
                    field_name,
                    e.valid_up_to()
                ))
            })
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        stream.write_all(self.value.as_bytes()).await?;
        stream.write_u8(0u8).await
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }
}

fn inv(message: String) -> io::Error {
    io::Error::new::<String>(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_integer1() {
        let mut bytes = io::BufReader::new(&[0x23][..]);
        assert_eq!(Integer1::read(&mut bytes).unwrap(), Integer1::new(0x23));
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
            COctetString::read(&mut bytes, 20, "test_field").unwrap(),
            COctetString::new(AsciiStr::from_ascii("foobar").unwrap(), 20)
        );
    }

    #[tokio::test]
    async fn write_coctetstring() {
        let mut buf: Vec<u8> = Vec::new();
        let val = COctetString::new(AsciiStr::from_ascii("abc").unwrap(), 16);
        val.write(&mut buf).await.unwrap();
        assert_eq!(buf, vec!['a' as u8, 'b' as u8, 'c' as u8, 0x00]);
    }

    // TODO: read error
    // TODO: end of stream before end of string
    // TODO: missing \0
    // TODO: coctetstring too long
    // TODO: coctetstring exactly right length
}
