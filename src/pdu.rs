use std::io::Cursor;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::result::Result;

#[derive(Debug)]
pub enum Pdu {
    BindTransmitter(BindTransmitterPdu),
    BindTransmitterResp(BindTransmitterRespPdu),
}

pub enum CheckResult {
    Ok,
    Incomplete,
    Error(Box<dyn std::error::Error + Send + Sync>),
}

impl Pdu {
    pub fn parse(bytes: &mut Cursor<&[u8]>) -> Result<Pdu> {
        //let length = self.tcp_stream.read_u32().await?;

        Ok(Pdu::BindTransmitter(BindTransmitterPdu {
            sequence_number: 0x12,
            system_id: String::from(""),
            password: String::from(""),
            system_type: String::from(""),
            interface_version: 0x99,
            addr_ton: 0x99,
            addr_npi: 0x99,
            address_range: String::from("rng"),
        }))
    }

    pub fn check(bytes: &mut Cursor<&[u8]>) -> CheckResult {
        CheckResult::Ok
    }

    pub async fn write(&self, tcp_stream: &mut TcpStream) -> Result<()> {
        match self {
            Pdu::BindTransmitter(pdu) => pdu.write(tcp_stream).await,
            Pdu::BindTransmitterResp(pdu) => pdu.write(tcp_stream).await,
        }
    }
}

#[derive(Debug)]
pub struct BindTransmitterPdu {
    pub sequence_number: u32,
    pub system_id: String,
    pub password: String,
    pub system_type: String,
    pub interface_version: u8,
    pub addr_ton: u8,
    pub addr_npi: u8,
    pub address_range: String,
}

impl BindTransmitterPdu {
    async fn write(&self, _tcp_stream: &mut TcpStream) -> Result<()> {
        todo!()
    }
}

#[derive(Debug)]
pub struct BindTransmitterRespPdu {
    pub sequence_number: u32,
    pub system_id: String,
}

impl BindTransmitterRespPdu {
    async fn write(&self, tcp_stream: &mut TcpStream) -> Result<()> {
        // TODO: check max length
        tcp_stream
            .write_u32((16 + self.system_id.len() + 1) as u32)
            .await?; // length
        tcp_stream.write_u32(0x80000002).await?; // command_id: bind_transmitter_resp
        tcp_stream.write_u32(0).await?; // command_status
        tcp_stream.write_u32(self.sequence_number).await?;
        // TODO: check allowed characters (on creation and/or here)
        tcp_stream.write_all(self.system_id.as_bytes()).await?;
        tcp_stream.write_u8(0x00).await?;
        Ok(())
    }
}
