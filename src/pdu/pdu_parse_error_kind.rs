#[derive(Debug, PartialEq)]
pub enum PduParseErrorKind {
    NonasciiCOctetString,
}
