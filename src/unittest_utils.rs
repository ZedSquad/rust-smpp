#![cfg(test)]

use std::io;

pub struct FailingRead {}

impl FailingRead {
    pub fn new_bufreader() -> io::BufReader<FailingRead> {
        io::BufReader::new(FailingRead {})
    }

    fn error() -> io::Error {
        io::Error::from_raw_os_error(22)
    }

    pub fn error_string() -> String {
        FailingRead::error().to_string()
    }
}

impl io::Read for FailingRead {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(FailingRead::error())
    }
}
