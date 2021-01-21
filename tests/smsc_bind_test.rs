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

// TODO: partial PDU provided (with length implying longer)
// TODO: too-short length
// TODO: too-long length
// TODO: very very long length
// TODO: very very long octet string even though length claims it's less
