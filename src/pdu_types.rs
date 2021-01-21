use std::io;
use std::io::Read;

// TODO: PDU Types, from spec section 3.1 e.g.:
// Integer4
// Integer1
// COctetString
// COctetStringDecimal
// COctetStringHex
// OctetString

pub trait ReadSelf {
    fn read_self(bytes: &mut dyn Read) -> io::Result<Self>
    where
        Self: Sized;
}

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
}

impl ReadSelf for Integer1 {
    fn read_self(bytes: &mut dyn Read) -> io::Result<Self> {
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
    pub fn new(value: u32) -> Self {
        Self { value }
    }
}

impl ReadSelf for Integer4 {
    fn read_self(bytes: &mut dyn Read) -> io::Result<Self> {
        let mut ret: [u8; 4] = [0; 4];
        bytes.read_exact(&mut ret)?;
        Ok(Self {
            value: u32::from_be_bytes(ret),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_integer1() {
        let mut bytes = io::BufReader::new(&[0x23][..]);
        assert_eq!(
            Integer1::read_self(&mut bytes).unwrap(),
            Integer1::new(0x23)
        );
    }

    #[test]
    fn read_integer4() {
        let mut bytes = io::BufReader::new(&[0xf0, 0x00, 0x00, 0x23][..]);
        assert_eq!(
            Integer4::read_self(&mut bytes).unwrap(),
            Integer4::new(0xf0000023)
        );
    }
}
