use num_traits::{FromPrimitive, ToPrimitive};
use std::io;
use std::io::BufRead;
use tokio::io::AsyncWriteExt;

use crate::pdu::formats::WriteStream;

#[repr(u16)]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive)]
pub enum KnownTlvTag {
    dest_addr_subunit = 0x0005,
    dest_network_type = 0x0006,
    dest_bearer_type = 0x0007,
    dest_telematics_id = 0x0008,
    source_addr_subunit = 0x000D,
    source_network_type = 0x000E,
    source_bearer_type = 0x000F,
    source_telematics_id = 0x0010,
    qos_time_to_live = 0x0017,
    payload_type = 0x0019,
    additional_status_info_text = 0x001D,
    receipted_message_id = 0x001E,
    ms_msg_wait_facilities = 0x0030,
    privacy_indicator = 0x0201,
    source_subaddress = 0x0202,
    dest_subaddress = 0x0203,
    user_message_reference = 0x0204,
    user_response_code = 0x0205,
    source_port = 0x020A,
    destination_port = 0x020B,
    sar_msg_ref_num = 0x020C,
    language_indicator = 0x020D,
    sar_total_segments = 0x020E,
    sar_segment_seqnum = 0x020F,
    SC_interface_version = 0x0210,
    callback_num_pres_ind = 0x0302,
    callback_num_atag = 0x0303,
    number_of_messages = 0x0304,
    callback_num = 0x0381,
    dpf_result = 0x0420,
    set_dpf = 0x0421,
    ms_availability_status = 0x0422,
    network_error_code = 0x0423,
    message_payload = 0x0424,
    delivery_failure_reason = 0x0425,
    more_messages_to_send = 0x0426,
    message_state = 0x0427,
    Genericussd_service_op = 0x0501,
    display_time = 0x1201,
    sms_signal = 0x1203,
    ms_validity = 0x1204,
    alert_on_message_delivery = 0x130C,
    its_reply_type = 0x1380,
    its_session_info = 0x1383,
}

impl KnownTlvTag {
    /// Create a new KnownTlvTag from a raw tag code.  Returns None
    /// if this tag is not known.
    pub fn new(tag: u16) -> Option<Self> {
        FromPrimitive::from_u16(tag)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tlv {
    pub raw_tag: u16,
    pub value: Vec<u8>,
}

impl Tlv {
    pub fn new(tag: KnownTlvTag, value: &[u8]) -> Self {
        Self {
            // TlvTag is repr(u16) so unwrap will not fail
            raw_tag: ToPrimitive::to_u16(&tag).unwrap(),
            value: Vec::from(value),
        }
    }

    pub fn new_unknown(raw_tag: u16, value: &[u8]) -> Self {
        Self {
            raw_tag,
            value: Vec::from(value),
        }
    }

    /// Read the next TLV from the supplied BufRead.  If the BufRead is
    /// empty, return Ok(None).  If reading succeeds, return Ok(Some(Tlv)).
    /// If reading fails, return the relevant io::Error.
    pub fn read(bytes: &mut dyn BufRead) -> io::Result<Option<Self>> {
        let mut tag_bytes: [u8; 2] = [0; 2];
        let len = bytes.read(&mut tag_bytes[0..1])?;
        if len == 0 {
            return Ok(None);
        }

        bytes.read_exact(&mut tag_bytes[1..2])?;
        let raw_tag = u16::from_be_bytes(tag_bytes);

        let mut len_bytes: [u8; 2] = [0; 2];
        bytes.read_exact(&mut len_bytes)?;
        let length: usize = u16::from_be_bytes(len_bytes).into();
        // Later: Issue#20 - validate length.  For now, since it's a u16,
        // we know it is <65536, so not big enough to blow memory.

        let mut value = Vec::with_capacity(length);
        value.resize(length, 0);
        bytes.read_exact(&mut value[..])?;

        Ok(Some(Self { raw_tag, value }))
    }

    /// If this TLV's tag is known, return Ok() containing it.  Otherwise,
    /// return Err() containing the raw tag value.
    pub fn tag(&self) -> Result<KnownTlvTag, u16> {
        KnownTlvTag::new(self.raw_tag).ok_or(self.raw_tag)
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        stream.write_u16(self.raw_tag).await?;
        stream.write_u16(self.value.len() as u16).await?;
        stream.write(&self.value).await?;
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Tlvs {
    values: Vec<Tlv>,
}

impl Tlvs {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn from(tlvs: &[Tlv]) -> Self {
        Self {
            values: Vec::from(tlvs),
        }
    }

    pub fn read(bytes: &mut dyn BufRead) -> io::Result<Self> {
        let mut values = Vec::new();

        loop {
            let tlv = Tlv::read(bytes)?;
            if let Some(tlv) = tlv {
                values.push(tlv);
            } else {
                break;
            }
        }

        Ok(Self { values })
    }

    pub async fn write(&self, stream: &mut WriteStream) -> io::Result<()> {
        for tlv in &self.values {
            tlv.write(stream).await?;
        }
        Ok(())
    }

    pub fn get(&self, tag: KnownTlvTag) -> Option<Tlv> {
        self.get_unknown(ToPrimitive::to_u16(&tag).unwrap())
    }

    pub fn get_unknown(&self, tag: u16) -> Option<Tlv> {
        self.values
            .iter()
            .filter(|&tlv| tlv.raw_tag == tag)
            .next()
            .map(Tlv::clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unittest_utils::FailingRead;

    #[test]
    fn read_message_payload_tlv() {
        let bytes = [0x04, 0x24, 0x00, 0x03, 0x61, 0x62, 0x63];
        let mut reader = io::BufReader::new(&bytes[..]);
        let tlv = Tlv::read(&mut reader).unwrap().unwrap();

        assert_eq!(
            tlv,
            Tlv::new(KnownTlvTag::message_payload, "abc".as_bytes())
        );
        assert_eq!(tlv.tag(), Ok(KnownTlvTag::message_payload));
    }

    #[test]
    fn read_tlv_with_unknown_tag_works() {
        let bytes = [0xff, 0x24, 0x00, 0x03, 0x61, 0x62, 0x63];
        let mut reader = io::BufReader::new(&bytes[..]);
        let tlv = Tlv::read(&mut reader).unwrap().unwrap();

        assert_eq!(tlv, Tlv::new_unknown(0xff24, "abc".as_bytes()));
        assert_eq!(tlv.tag(), Err(0xff24));
    }

    #[test]
    fn read_tlv_with_length_longer_than_input_fails() {
        let bytes = [0xff, 0x24, 0x00, 0x04, 0x61, 0x62, 0x63];
        let mut reader = io::BufReader::new(&bytes[..]);
        let err = Tlv::read(&mut reader).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn error_reading_tlv_is_surfaced() {
        let mut failing_read = FailingRead::new_bufreader();
        let res = Tlv::read(&mut failing_read).unwrap_err();
        assert_eq!(res.to_string(), FailingRead::error_string());
    }

    #[tokio::test]
    async fn write_tlv_with_known_tag() {
        let mut buf: Vec<u8> = Vec::new();
        Tlv::new(KnownTlvTag::message_payload, &[0, 1, 2, 3])
            .write(&mut buf)
            .await
            .unwrap();
        assert_eq!(buf, vec![0x04, 0x24, 0x00, 0x04, 0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn write_tlv_with_unknown_tag() {
        let mut buf: Vec<u8> = Vec::new();
        Tlv::new_unknown(0xffff, &[0, 1, 2, 3])
            .write(&mut buf)
            .await
            .unwrap();
        assert_eq!(buf, vec![0xff, 0xff, 0x00, 0x04, 0, 1, 2, 3]);
    }

    #[test]
    fn read_multiple_tlvs() {
        let bytes = [
            0x04, 0x24, 0x00, 0x03, 0x61, 0x62, 0x63, 0xff, 0x24, 0x00, 0x03,
            0x61, 0x62, 0x63,
        ];
        let mut reader = io::BufReader::new(&bytes[..]);
        let tlvs = Tlvs::read(&mut reader).unwrap();

        assert_eq!(
            tlvs,
            Tlvs::from(&[
                Tlv::new(KnownTlvTag::message_payload, "abc".as_bytes()),
                Tlv::new_unknown(0xff24, "abc".as_bytes()),
            ])
        );
    }

    #[test]
    fn reading_malformed_tlvs_is_an_error() {
        // First TLV's length is too long
        let bytes = [
            0x04, 0x24, 0x00, 0x13, 0x61, 0x62, 0x63, 0xff, 0x24, 0x00, 0x03,
            0x61, 0x62, 0x63,
        ];
        let mut reader = io::BufReader::new(&bytes[..]);
        let tlvs = Tlvs::read(&mut reader).unwrap_err();

        assert_eq!(tlvs.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn io_error_reading_tlvs_is_surfaced() {
        let mut failing_read = FailingRead::new_bufreader();
        let tlvs = Tlvs::read(&mut failing_read).unwrap_err();

        assert_eq!(tlvs.to_string(), FailingRead::error_string());
    }

    #[tokio::test]
    async fn write_multiple_tlvs() {
        let mut buf: Vec<u8> = Vec::new();
        let tlvs = Tlvs::from(&[
            Tlv::new_unknown(0xffff, &[0, 1, 2, 3]),
            Tlv::new(KnownTlvTag::sms_signal, &[89]),
        ]);
        tlvs.write(&mut buf).await.unwrap();
        assert_eq!(
            buf,
            vec![
                0xff, 0xff, 0x00, 0x04, 0, 1, 2, 3, 0x12, 0x03, 0x00, 0x01, 89,
            ]
        );
    }

    #[test]
    fn can_get_existing_tlv_from_list() {
        let tlv1 = Tlv::new(KnownTlvTag::sms_signal, &[1]);
        let tlv2 = Tlv::new(KnownTlvTag::message_payload, "abc".as_bytes());
        let tlv3 = Tlv::new_unknown(0xffff, &[3, 2]);
        let tlvs = Tlvs::from(&[tlv1.clone(), tlv2.clone(), tlv3.clone()]);
        assert_eq!(tlvs.get(KnownTlvTag::message_payload), Some(tlv2));
        assert_eq!(tlvs.get_unknown(0xffff), Some(tlv3));
    }

    #[test]
    fn getting_nonexistent_tlv_returns_none() {
        let tlv1 = Tlv::new(KnownTlvTag::sms_signal, &[1]);
        let tlv2 = Tlv::new(KnownTlvTag::message_payload, "abc".as_bytes());
        let tlv3 = Tlv::new_unknown(0xffff, &[3, 2]);
        let tlvs = Tlvs::from(&[tlv1.clone(), tlv2.clone(), tlv3.clone()]);
        assert_eq!(tlvs.get(KnownTlvTag::dpf_result), None);
    }
}
