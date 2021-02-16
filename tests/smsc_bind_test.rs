use std::io;
use std::iter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod test_utils;

use test_utils::{
    bytes_as_string as s, TestClient, TestServer, BIND_RECEIVER_PDU,
    BIND_RECEIVER_RESP_PDU, BIND_TRANSCEIVER_PDU, BIND_TRANSCEIVER_RESP_PDU,
    BIND_TRANSMITTER_PDU, BIND_TRANSMITTER_RESP_PDU,
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
fn when_we_receive_bind_receiver_we_respond_with_resp() {
    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME binds
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(BIND_RECEIVER_PDU).await.unwrap();

        // Then SMSC responds correctly
        let resp = client.read_n(BIND_RECEIVER_RESP_PDU.len()).await;
        assert_eq!(s(&resp), s(BIND_RECEIVER_RESP_PDU));
    })
}

#[test]
fn when_we_receive_bind_transceiver_we_respond_with_resp() {
    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME binds
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(BIND_TRANSCEIVER_PDU).await.unwrap();

        // Then SMSC responds correctly
        let resp = client.read_n(BIND_TRANSCEIVER_RESP_PDU.len()).await;
        assert_eq!(s(&resp), s(BIND_TRANSCEIVER_RESP_PDU));
    })
}

#[test]
fn when_we_receive_enquire_link_we_respond_with_resp() {
    const ENQUIRE_LINK: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x00\x00\x00\x15\x00\x00\x00\x00\x00\x00\x00\x12";

    const ENQUIRE_LINK_RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x15\x00\x00\x00\x00\x00\x00\x00\x12";

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When ESME enquires
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(ENQUIRE_LINK).await.unwrap();

        // Then SMSC responds, with the same sequence number
        let resp = client.read_n(ENQUIRE_LINK_RESP.len()).await;
        assert_eq!(s(&resp), s(ENQUIRE_LINK_RESP));
    })
}

#[test]
fn when_we_receive_a_bad_pdu_we_respond_with_failure_resp_pdu() {
    const PDU: &[u8; 0x29] =
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x14\
        e\xf0\x9f\x92\xa9d\0password\0type\0\x34\x00\x00\0";
    //  ^^^^ non-ascii

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x08\x00\x00\x00\x14";
    //                                   system error ^^^^        seq ^^^^
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
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x01";
    //       generic_nack ^^^^        invalid cmd len ^^^^        seq ^^^^

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
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x02";
    //       generic_nack ^^^^          invalid cmdid ^^^^        seq ^^^^

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
        b"\x00\x00\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        esmeid\0password\0type\0\x34\x00\x00\0";
    // length longer than content

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x02\x00\x00\x00\x02";
    //          bind_transmitter_resp ^^^^              ^^ cmd len invalid

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
        b"\x00\xff\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        e\0pd\0t\0\x34\x00\x00\0";
    // very long length

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x01";
    //       generic_nack ^^^^        cmd len invalid ^^^^        seq ^^^^

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

        // Then SMSC either ...
        let resp = client.read_n_maybe(RESP.len()).await;

        match resp {
            // responds with an error then drops the connection
            Ok(resp) => {
                assert_eq!(s(&resp), s(RESP));
                assert_eq!(
                    client.stream.read_u8().await.unwrap_err().kind(),
                    io::ErrorKind::ConnectionReset
                );
            }
            // or drops the connection immediately
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::ConnectionReset);
            }
        }
    })
}

#[test]
fn when_receive_pdu_with_short_length_but_long_string_we_respond_with_error() {
    const BEGIN: &[u8; 0x11] =
        b"\x00\x00\x00\x1b\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02e";
    const END: &[u8; 0x0a] = b"\0pd\0t\0\x34\x00\x00\0";

    const RESP: &[u8; 0x10] =
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x08\x00\x00\x00\x02";
    //          bind_transmitter_resp ^^^^ system error ^^

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
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x02";
    //       generic_nack ^^^^          invalid cmdid ^^^^        seq ^^^^

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
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x22";
    //       generic_nack ^^^^          invalid cmdid ^^^^        seq ^^^^

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

#[test]
fn when_we_receive_submit_sm_we_respond_with_resp() {
    let mut pdu: Vec<u8> = Vec::new();
    pdu.extend(b"\x00\x00\x00\x3d"); //   command_length = 61
    pdu.extend(b"\x00\x00\x00\x04"); //       command_id = submit_sm
    pdu.extend(b"\x00\x00\x00\x00"); //   command_status = NULL
    pdu.extend(b"\x00\x00\x00\x03"); //  sequence_number = 3
    pdu.extend(b"\x00"); //                 service_type = 0
    pdu.extend(b"\x00"); //               source_add_ton = 0
    pdu.extend(b"\x00"); //              source_addr_npi = 0
    pdu.extend(b"447000123123\x00"); //      source_addr
    pdu.extend(b"\x00"); //                 dest_add_ton = 0
    pdu.extend(b"\x00"); //                dest_addr_npi = 0
    pdu.extend(b"447111222222\x00"); // destination_addr
    pdu.extend(b"\x00"); //                    esm_class = 0
    pdu.extend(b"\x01"); //                  protocol_id = 1
    pdu.extend(b"\x01"); //                priority_flag = 1
    pdu.extend(b"\x00"); //       schedule_delivery_time = 0
    pdu.extend(b"\x00"); //              validity_period = 0
    pdu.extend(b"\x01"); //          registered_delivery = 1
    pdu.extend(b"\x00"); //      replace_if_present_flag = 0
    pdu.extend(b"\x03"); //                  data_coding = 3
    pdu.extend(b"\x00"); //            sm_default_msg_id = 0
    pdu.extend(b"\x04"); //                    sm_length = 4
    pdu.extend(b"hihi"); //                short_message = hihi

    let mut resp: Vec<u8> = Vec::new();
    resp.extend(b"\x00\x00\x00\x11"); //  command_length = 17
    resp.extend(b"\x80\x00\x00\x04"); //      command_id = submit_sm_resp
    resp.extend(b"\x00\x00\x00\x00"); //  command_status = ESME_ROK
    resp.extend(b"\x00\x00\x00\x03"); // sequence_number = 3
    resp.extend(b"\x00"); //                  message_id = ""

    // Given an SMSC
    let server = TestServer::start().unwrap();
    server.runtime.block_on(async {
        // When we send a submit_sm
        let mut client = TestClient::connect_to(&server).await.unwrap();
        client.stream.write(&pdu).await.unwrap();

        // Then SMSC responds, with the same sequence number
        let resp = client.read_n(resp.len()).await;
        assert_eq!(s(&resp), s(&resp));
    })
}

// TODO: allow and disallow binding via username+password (pluggable validator)
// TODO: receive MT (pluggable handler)
// TODO: return DR
// TODO: return MO
// Later: client app + system test that allows us to compare with CloudHopper
// Later: different bind types
// Later: all PDU types and formats
