use tokio::io::AsyncWriteExt;

mod test_utils;

use test_utils::{bytes_as_string as s, TestClient, TestServer};

const BIND_TRANSMITTER_PDU: &[u8; 0x29] =
    b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x01esmeid\0password\0type\0\x34\x00\x00\0";

const BIND_TRANSMITTER_RESP_PDU: &[u8; 0x1b] =
    b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x01TestServer\0";

#[test]
fn responds_to_bind_transmitter() {
    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME binds
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(BIND_TRANSMITTER_PDU).await.unwrap();
        // Then SMSC responds
        let resp = client.read_n(BIND_TRANSMITTER_RESP_PDU.len()).await;
        assert_eq!(s(&resp), s(BIND_TRANSMITTER_RESP_PDU));
    })
}
