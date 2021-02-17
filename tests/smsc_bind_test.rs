use std::io;
use std::iter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use smpp::smsc::{BindData, BindError, SmscLogic};

mod test_utils;

use test_utils::{bytes_as_string, TestClient, TestServer};

#[test]
fn when_we_receive_bind_transmitter_we_respond_with_resp() {
    // Given a server with a client connected to it
    TestSetup::new().send_and_expect_response(
        // When client sends bind_transmitter, sequence_number = 2
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // Then server responds bind_transmitter_resp, sequence_number = 2
        b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        TestServer\0",
    );
}

#[test]
fn when_we_receive_bind_receiver_we_respond_with_resp() {
    TestSetup::new().send_and_expect_response(
        // When client sends bind_receiver, sequence_number = 8
        b"\x00\x00\x00\x29\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x08\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // Then server responds bind_receiver_resp, sequence_number = 8
        b"\x00\x00\x00\x1b\x80\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x08\
        TestServer\0",
    );
}

#[test]
fn when_we_receive_bind_transceiver_we_respond_with_resp() {
    TestSetup::new().send_and_expect_response(
        // When client sends bind_transceiver, sequence_number = 6
        b"\x00\x00\x00\x29\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // Then server responds bind_transceiver_resp, sequence_number = 6
        b"\x00\x00\x00\x1b\x80\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        TestServer\0",
    );
}

#[test]
fn when_we_bind_with_incorrect_password_we_receive_error() {
    struct PwIsAlwaysWrong {}
    impl SmscLogic for PwIsAlwaysWrong {
        fn bind(&self, _bind_data: &BindData) -> Result<(), BindError> {
            Err(BindError::IncorrectPassword)
        }
    }

    let logic = PwIsAlwaysWrong {};

    let t = TestSetup::new_with_logic(logic);
    t.send_and_expect_response(
        // bind_transceiver
        b"\x00\x00\x00\x29\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // command_status=ESME_RINVPASWD
        b"\x00\x00\x00\x10\x80\x00\x00\x09\x00\x00\x00\x0e\x00\x00\x00\x06",
    );
    t.send_and_expect_response(
        // bind_receiver
        b"\x00\x00\x00\x29\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // command_status=ESME_RINVPASWD
        b"\x00\x00\x00\x10\x80\x00\x00\x01\x00\x00\x00\x0e\x00\x00\x00\x06",
    );
    t.send_and_expect_response(
        // bind_transmitter
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // command_status=ESME_RINVPASWD
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x0e\x00\x00\x00\x06",
    );
}

#[test]
fn when_we_receive_enquire_link_we_respond_with_resp() {
    TestSetup::new().send_and_expect_response(
        // When client sends enquire_link
        b"\x00\x00\x00\x10\x00\x00\x00\x15\x00\x00\x00\x00\x00\x00\x00\x12",
        // Then server responds enquire_link_resp
        b"\x00\x00\x00\x10\x80\x00\x00\x15\x00\x00\x00\x00\x00\x00\x00\x12",
    );
}

#[test]
fn when_we_receive_a_bad_pdu_we_respond_with_failure_resp_pdu() {
    TestSetup::new().send_and_expect_error_response(
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x14\
        e\xf0\x9f\x92\xa9d\0password\0type\0\x34\x00\x00\0",
        //  ^^^^ non-ascii
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x08\x00\x00\x00\x14",
        //                               system error ^^^^        seq ^^^^
        // Note: no body part because this is an error response
        "unexpected end of file",
    );
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
    TestSetup::new().send_and_expect_error_response(
        b"\x00\x00\x00\x01",
        // length is 1! ^^
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x01",
        //     generic_nack ^^^^            ^^^ invalid cmd len   seq ^^^^
        "unexpected end of file",
    );
}

#[test]
fn when_we_receive_wrong_type_of_pdu_we_respond_generic_nack() {
    TestSetup::new().send_and_expect_error_response(
        b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        TestServer\0",
        // bind_transmitter_resp ^^^^^^^^^^^^^ - doesn't make sense
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x02",
        //       generic_nack ^^^^          invalid cmdid ^^^^        seq ^^^^
        "unexpected end of file",
    );
}

#[test]
fn when_we_receive_nontlv_pdu_with_too_long_length_return_an_error() {
    const PDU: &[u8; 0x29] =
        b"\x00\x00\xff\xff\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        esmeid\0password\0type\0\x34\x00\x00\0";
    // length longer than content

    let many_bytes: Vec<u8> = PDU
        .iter()
        .copied()
        .chain(iter::repeat(0x00))
        .take(100_000)
        .collect();

    TestSetup::new().send_and_expect_error_response(
        &many_bytes,
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x02\x00\x00\x00\x02",
        //      bind_transmitter_resp ^^^^              ^^ cmd len invalid
        "Connection reset by peer (os error 104)",
    );
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
                assert_eq!(bytes_as_string(&resp), bytes_as_string(RESP));
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

    // Our PDU will contain 100,000 letter 'e's within a COctetString
    let mut many_bytes: Vec<u8> = vec![];
    many_bytes.extend(BEGIN.iter());
    many_bytes.extend(iter::repeat('e' as u8).take(100_000));
    many_bytes.extend(END.iter());

    TestSetup::new().send_and_expect_error_response(
        &many_bytes,
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x08\x00\x00\x00\x02",
        //      bind_transmitter_resp ^^^^ system error ^^
        "Connection reset by peer (os error 104)",
    );
}

#[test]
fn when_we_receive_invalid_pdu_type_we_respond_with_error() {
    TestSetup::new().send_and_expect_error_response(
        b"\x00\x00\x00\x10\xff\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x22",
        //    this is invalid! ^^^^^^^^^^^^^^^                    seq ^^^^
        b"\x00\x00\x00\x10\x80\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x22",
        //   generic_nack ^^^^          invalid cmdid ^^^^        seq ^^^^
        "unexpected end of file",
    );
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

    TestSetup::new().send_and_expect_response(&pdu, &resp);
}

/// Setup for running tests that send and receive PDUs
pub struct TestSetup {
    server: TestServer,
}

impl TestSetup {
    pub fn new() -> Self {
        let server = TestServer::start().unwrap();
        Self { server }
    }

    fn new_with_logic<L: SmscLogic + Send + 'static>(smsc_logic: L) -> Self {
        let server = TestServer::start_with_logic(smsc_logic).unwrap();
        Self { server }
    }

    async fn send_exp(
        &self,
        input: &[u8],
        expected_output: &[u8],
    ) -> TestClient {
        let mut client = TestClient::connect_to(&self.server).await.unwrap();
        client.stream.write(input).await.unwrap();

        let resp = client.read_n(expected_output.len()).await;
        assert_eq!(bytes_as_string(&resp), bytes_as_string(expected_output));
        client
    }

    pub fn send_and_expect_response(
        &self,
        input: &[u8],
        expected_output: &[u8],
    ) {
        self.server.runtime.block_on(async {
            self.send_exp(input, expected_output).await;
        })
    }

    pub fn send_and_expect_error_response(
        &self,
        input: &[u8],
        expected_output: &[u8],
        expected_error: &str,
    ) {
        self.server.runtime.block_on(async {
            let mut client = self.send_exp(input, expected_output).await;

            // Since this is an error, server should drop the connection
            let resp = client.stream.read_u8().await.unwrap_err();
            assert_eq!(&resp.to_string(), expected_error);
        })
    }
}

// TODO: allow and disallow binding via username+password (pluggable validator)
// TODO: receive MT (pluggable handler)
// TODO: return DR
// TODO: return MO
// Later: client app + system test that allows us to compare with CloudHopper
// Later: smpp session states (spec 2.2)
// Later: sc_interface_version TLV in bind response
// Later: Check interface versions in binds and responses, and submit_sm
// Later: all PDU types and formats
