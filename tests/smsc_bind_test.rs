use async_trait::async_trait;

use smpp::smsc::{BindData, BindError, SmscLogic};

mod test_utils;

use test_utils::TestSetup;

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

    #[async_trait]
    impl SmscLogic for PwIsAlwaysWrong {
        async fn bind(&self, _bind_data: &BindData) -> Result<(), BindError> {
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

// TODO: allow and disallow binding via username+password (pluggable validator)
// TODO: receive MT (pluggable handler)
// TODO: return DR
// TODO: return MO
// Later: client app + system test that allows us to compare with CloudHopper
// Later: smpp session states (spec 2.2)
// Later: sc_interface_version TLV in bind response
// Later: Check interface versions in binds and responses, and submit_sm
// Later: all PDU types and formats
