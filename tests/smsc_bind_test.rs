use std::iter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod test_utils;

use test_utils::{
    bytes_as_string as s, TestClient, TestServer, BIND_TRANSMITTER_PDU,
    BIND_TRANSMITTER_RESP_PDU,
};

#[test]
fn when_we_receive_bind_transmitter_we_respond_with_resp() {
    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME binds with:
        // * sequence number = 2
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(BIND_TRANSMITTER_PDU).await.unwrap();

        // Then SMSC responds, with:
        // * length = 1b
        // * type   80000002
        // * status 0 (because all is OK)
        // * sequence number = 2 (because that is what we provided)
        // * system_id = TestServer (as set up in TestServer)
        let resp = client.read_n(BIND_TRANSMITTER_RESP_PDU.len()).await;
        assert_eq!(s(&resp), s(BIND_TRANSMITTER_RESP_PDU));
    })
}

#[test]
fn when_we_receive_a_bad_pdu_we_respond_with_failure_resp_pdu() {
    const PDU: &[u8; 0x29] =
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x14e\xf0\x9f\x92\xa9d\0password\0type\0\x34\x00\x00\0";
    //                                                          non-ascii  ^^^^

    const RESP: &[u8; 0x10] =
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

        // And then drops the connection
        let resp = client.stream.read_u8().await.unwrap_err();
        assert_eq!(&resp.to_string(), "unexpected end of file");
    })
}

#[test]
fn when_client_disconnects_within_pdu_we_continue_accepting_new_connections() {
    const PDU: &[u8; 0x11] =
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x14e";

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME sends partial data then disconnects
        let mut client1 = TestClient::connect_to(&server).await.unwrap();
        client1.stream.write(PDU).await.unwrap();
        client1.stream.shutdown().await.unwrap();

        // Another client is free to connect afterwards
        TestClient::connect_to(&server).await.unwrap();
    })
}

#[test]
fn when_sent_bad_pdu_header_we_respond_generic_nack() {
    const PDU: &[u8; 0x04] = b"\x00\x00\x00\x01";
    //                        length is 1! ^^^^

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x01\x00\x02\x00\x00\x00\x00";
    //       generic_nack ^^^^^^^^^^^^^^^^      error ^^^^        seq ^^^^

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME tries to bind with a PDU with invalid header
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(PDU).await.unwrap();

        // Then SMSC responds with an error response
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));

        // And then drops the connection
        let resp = client.stream.read_u8().await.unwrap_err();
        assert_eq!(&resp.to_string(), "unexpected end of file");
    })
}

#[test]
fn when_we_receive_wrong_type_of_pdu_we_respond_generic_nack() {
    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x01\x00\x03\x00\x00\x00\x02";
    //       generic_nack ^^^^^^^^^^^^^^^^      error ^^^^        seq ^^^^

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME sends a BIND_TRANSMITTER_RESP, which does not make sense
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client
            .stream
            .write(BIND_TRANSMITTER_RESP_PDU)
            .await
            .unwrap();

        // Then SMSC responds with an error response
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));

        // And then drops the connection
        let resp = client.stream.read_u8().await.unwrap_err();
        assert_eq!(&resp.to_string(), "unexpected end of file");
    })
}

#[test]
fn when_we_receive_nontlv_pdu_with_too_long_length_return_an_error() {
    const PDU: &[u8; 0x29] =
        b"\x00\x00\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02esmeid\0password\0type\0\x34\x00\x00\0";
    //            ^^^^^^^^ length longer than content

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x01\x00\x00\x00\x02";
    //          bind_transmitter_resp ^^^^

    let many_bytes: Vec<u8> = PDU
        .iter()
        .copied()
        .chain(iter::repeat(0x00))
        .take(100_000)
        .collect();

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When we send a short PDU with a length value suggesting it is long
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(&many_bytes).await.unwrap();

        // Then SMSC responds with bind_transmitter_resp error
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));

        // And then drops the connection
        let resp = client.stream.read_u8().await.unwrap_err();
        assert_eq!(
            &resp.to_string(),
            "Connection reset by peer (os error 104)"
        );
    })
}

#[test]
fn when_we_receive_a_pdu_with_very_long_length_we_respond_generic_nack() {
    const PDU: &[u8; 0x1b] =
        b"\x00\xff\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02e\0pd\0t\0\x34\x00\x00\0";
    //            ^^^^^^^^ very long length

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x01\x00\x02\x00\x00\x00\x00";
    //       generic_nack ^^^^^^^^^^^^^^^^      error ^^^^        seq ^^^^

    // Note: we don't provide the correct sequence number here: we could, but
    // we would have to read the PDU header before we notice the invalid
    // PDU length.  Since a huge length is likely to indicate a catastrophic
    // error, or malicious traffic, we are not too bothered.

    let many_bytes: Vec<u8> = PDU
        .iter()
        .copied()
        .chain(iter::repeat(0x00))
        .take(0x00ffffff)
        .collect();

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When we send a huge PDU with huge length
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(&many_bytes).await.unwrap();

        // Then SMSC responds with an error
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));

        // And then drops the connection
        let resp = client.stream.read_u8().await.unwrap_err();
        assert_eq!(
            &resp.to_string(),
            "Connection reset by peer (os error 104)"
        );
    })
}

#[test]
fn when_receive_pdu_with_short_length_but_long_string_we_respond_with_error() {
    const BEGIN: &[u8; 0x11] =
        b"\x00\x00\x00\x1b\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02e";
    const END: &[u8; 0x0a] = b"\0pd\0t\0\x34\x00\x00\0";

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x01\x00\x00\x00\x02";
    //          bind_transmitter_resp ^^^^

    // Our PDU will contain 100,000 letter 'e's within a COctetString
    let mut many_bytes: Vec<u8> = vec![];
    many_bytes.extend(BEGIN.iter());
    many_bytes.extend(iter::repeat('e' as u8).take(100_000));
    many_bytes.extend(END.iter());

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When we send a huge PDU with small length
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(&many_bytes).await.unwrap();

        // Then SMSC responds with a generic error
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));

        // And then drops the connection
        let resp = client.stream.read_u8().await.unwrap_err();
        assert_eq!(
            &resp.to_string(),
            "Connection reset by peer (os error 104)"
        );
    })
}

#[test]
fn when_we_receive_unexpected_pdu_type_we_respond_with_error() {
    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x01\x00\x03\x00\x00\x00\x02";
    //       generic_nack ^^^^^^^^^^^^^^^^      error ^^^^        seq ^^^^

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME binds with:
        // * sequence number = 2
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client
            .stream
            .write(BIND_TRANSMITTER_RESP_PDU)
            .await
            .unwrap();

        // Then SMSC responds, with:
        // * length = 1b
        // * type   80000000
        // * status 00010003 (because this is an error)
        // * sequence number = 00000002 (because that is what we provided)
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));
    })
}

#[test]
fn when_we_receive_invalid_pdu_type_we_respond_with_error() {
    const PDU: &[u8; 0x10] =
        b"\x00\x00\x00\x10\xff\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x22";
    //  invalid command_id ^^^^^^^^^^^^^^^^                       seq ^^^^

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x01\x00\x01\x00\x00\x00\x22";
    //       generic_nack ^^^^^^^^^^^^^^^^      error ^^^^        seq ^^^^

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME binds with:
        // * sequence number = 2
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(PDU).await.unwrap();

        // Then SMSC responds with an error
        let resp = client.read_n(RESP.len()).await;
        assert_eq!(s(&resp), s(RESP));
    })
}

// TODO: allow and disallow binding via username+password (pluggable validator)
// TODO: receive MT (pluggable handler)
// TODO: return DR
// TODO: return MO
// TODO: fix flakiness - sometimes we get Connection Refused when running this.
// Later: client app + system test that allows us to compare with CloudHopper
// Later: different bind types
// Later: all PDU types and formats
