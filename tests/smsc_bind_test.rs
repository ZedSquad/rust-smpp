use tokio::io::AsyncWriteExt;

mod test_utils;

use test_utils::{
    bytes_as_string as s, TestClient, TestServer, BIND_TRANSMITTER_PDU,
    BIND_TRANSMITTER_RESP_PDU,
};

#[test]
fn responds_to_bind_transmitter() {
    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME binds with:
        // * sequence number = 1
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(BIND_TRANSMITTER_PDU).await.unwrap();

        // Then SMSC responds, with:
        // * length = 1b
        // * type   80000002
        // * status 0 (because all is OK)
        // * sequence number = 1 (because that is what we provided)
        // * system_id = TestServer (as set up in TestServer)
        let resp = client.read_n(BIND_TRANSMITTER_RESP_PDU.len()).await;
        assert_eq!(s(&resp), s(BIND_TRANSMITTER_RESP_PDU));
    })
}

/* Guiding test, fails for now
#[test]
fn responds_failure_to_bad_pdu() {
    pub const PDU: &[u8; 0x29] =
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x14e\xf0\x9f\x92\xa9d\0password\0type\0\x34\x00\x00\0";

    pub const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x01\x00\x00\x00\x14";
    //                                          error ^^^^        seq ^^^^
    // Note: no body part because this is an error response

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME tries to bind with an invalid PDU
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(PDU).await.unwrap();

        // Then SMSC responds with an error response
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));
    })
}
*/

// TODO: change sequence number and pass without modifying hard-coded value
// TODO: partial PDU provided (with length implying longer)
// TODO: too-short length
// TODO: too-long length
// TODO: very very long length
// TODO: very very long octet string even though length claims it's less
