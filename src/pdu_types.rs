use ascii::{AsciiStr, AsciiString};
use std::io;
use std::io::{BufRead, Read};

// TODO: PDU Types, from spec section 3.1
// COctetStringDecimal
// COctetStringHex
// OctetString

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
    pub fn from(value: u8) -> Self {
        Self { value }
    }

    pub fn read(bytes: &mut dyn Read) -> io::Result<Self> {
        // Is allocating this buffer the right way?
        let mut ret: [u8; 1] = [0; 1];
        bytes.read_exact(&mut ret)?;
        Ok(Self { value: ret[0] })
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
    pub fn from(value: u32) -> Self {
        Self { value }
    }

    pub fn read(bytes: &mut dyn Read) -> io::Result<Self> {
        let mut ret: [u8; 4] = [0; 4];
        bytes.read_exact(&mut ret)?;
        Ok(Self {
            value: u32::from_be_bytes(ret),
        })
    }
}

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 3.1
///
/// C-Octet String:
/// A series of ASCII characters terminated with the NULL character.
#[derive(Debug, PartialEq)]
pub struct COctetString {
    pub value: AsciiString,
}

impl COctetString {
    pub fn from(value: &AsciiStr) -> Self {
        Self {
            value: AsciiString::from(value),
        }
    }

    pub fn read(
        bytes: &mut dyn BufRead,
        max_len: u64,
        field_name: &str,
    ) -> io::Result<Self> {
        let mut buf = Vec::new();
        bytes.take(max_len).read_until(0x00, &mut buf)?;

        if buf.last() != Some(&0x00) {
            // Failed to read a NULL terminator before we ran out of characters
            return Err(io::Error::new::<String>(
                io::ErrorKind::InvalidData,
                format!(
                    "String value for {} is too long.  Max length is {}.",
                    field_name, max_len
                )
                .into(),
            ));
        }

        let buf = &buf[..(buf.len() - 1)]; // Remove trailing 0 byte
        AsciiStr::from_ascii(buf)
            .map(|s| COctetString::from(s))
            .map_err(|e| {
                io::Error::new::<String>(
                    io::ErrorKind::InvalidData,
                    format!(
                    "String value of {} is not ASCII (valid up to byte {}).",
                    field_name, e.valid_up_to()
                )
                    .into(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_integer1() {
        let mut bytes = io::BufReader::new(&[0x23][..]);
        assert_eq!(Integer1::read(&mut bytes).unwrap(), Integer1::from(0x23));
    }

    #[test]
    fn read_integer4() {
        let mut bytes = io::BufReader::new(&[0xf0, 0x00, 0x00, 0x23][..]);
        assert_eq!(
            Integer4::read(&mut bytes).unwrap(),
            Integer4::from(0xf0000023)
        );
    }
}
