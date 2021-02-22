use async_trait::async_trait;

use smpp::pdu::{SubmitSmPdu, SubmitSmRespPdu};
use smpp::smsc::{BindData, BindError, SmscLogic, SubmitSmError};

mod test_utils;

use test_utils::TestSetup;

#[tokio::test]
async fn when_we_receive_bind_transmitter_we_respond_with_resp() {
    // Given a server with a client connected to it
    TestSetup::new()
        .await
        .send_and_expect_response(
            // When client sends bind_transmitter, sequence_number = 2
            b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        esmeid\0password\0type\0\x34\x00\x00\0",
            // Then server responds bind_transmitter_resp, sequence_number = 2
            b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x02\
        TestServer\0",
        )
        .await;
}

#[tokio::test]
async fn when_we_receive_bind_receiver_we_respond_with_resp() {
    TestSetup::new()
        .await
        .send_and_expect_response(
            // When client sends bind_receiver, sequence_number = 8
            b"\x00\x00\x00\x29\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x08\
        esmeid\0password\0type\0\x34\x00\x00\0",
            // Then server responds bind_receiver_resp, sequence_number = 8
            b"\x00\x00\x00\x1b\x80\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x08\
        TestServer\0",
        )
        .await;
}

#[tokio::test]
async fn when_we_receive_bind_transceiver_we_respond_with_resp() {
    TestSetup::new()
        .await
        .send_and_expect_response(
            // When client sends bind_transceiver, sequence_number = 6
            b"\x00\x00\x00\x29\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
            // Then server responds bind_transceiver_resp, sequence_number = 6
            b"\x00\x00\x00\x1b\x80\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        TestServer\0",
        )
        .await;
}

#[tokio::test]
async fn when_we_bind_with_incorrect_password_we_receive_error() {
    struct PwIsAlwaysWrong {}

    #[async_trait]
    impl SmscLogic for PwIsAlwaysWrong {
        async fn bind(
            &mut self,
            _bind_data: &BindData,
        ) -> Result<(), BindError> {
            Err(BindError::IncorrectPassword)
        }

        async fn submit_sm(
            &mut self,
            _pdu: &SubmitSmPdu,
        ) -> Result<SubmitSmRespPdu, SubmitSmError> {
            panic!("submit_sm not implemented");
        }
    }

    let logic = PwIsAlwaysWrong {};

    let t = TestSetup::new_with_logic(logic).await;
    t.send_and_expect_response(
        // bind_transceiver
        b"\x00\x00\x00\x29\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // command_status=ESME_RINVPASWD
        b"\x00\x00\x00\x10\x80\x00\x00\x09\x00\x00\x00\x0e\x00\x00\x00\x06",
    )
    .await;
    t.send_and_expect_response(
        // bind_receiver
        b"\x00\x00\x00\x29\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // command_status=ESME_RINVPASWD
        b"\x00\x00\x00\x10\x80\x00\x00\x01\x00\x00\x00\x0e\x00\x00\x00\x06",
    )
    .await;
    t.send_and_expect_response(
        // bind_transmitter
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        // command_status=ESME_RINVPASWD
        b"\x00\x00\x00\x10\x80\x00\x00\x02\x00\x00\x00\x0e\x00\x00\x00\x06",
    )
    .await;
}

#[tokio::test]
async fn when_we_receive_enquire_link_we_respond_with_resp() {
    TestSetup::new()
        .await
        .send_and_expect_response(
            // When client sends enquire_link
            b"\x00\x00\x00\x10\x00\x00\x00\x15\x00\x00\x00\x00\x00\x00\x00\x12",
            // Then server responds enquire_link_resp
            b"\x00\x00\x00\x10\x80\x00\x00\x15\x00\x00\x00\x00\x00\x00\x00\x12",
        )
        .await;
}

use std::sync::{Arc, Mutex};

#[tokio::test]
async fn when_we_receive_multiple_binds_we_can_keep_track() {
    struct TrackingLogic {
        num_binds: Arc<Mutex<u32>>,
    }

    #[async_trait]
    impl SmscLogic for TrackingLogic {
        async fn bind(
            &mut self,
            _bind_data: &BindData,
        ) -> Result<(), BindError> {
            *self.num_binds.lock().unwrap() += 1;
            println!("Bind number: {}", self.num_binds.lock().unwrap());
            Ok(())
        }

        async fn submit_sm(
            &mut self,
            _pdu: &SubmitSmPdu,
        ) -> Result<SubmitSmRespPdu, SubmitSmError> {
            panic!("submit_sm not implemented");
        }
    }

    let num_binds = Arc::new(Mutex::new(0));
    let logic = TrackingLogic {
        num_binds: Arc::clone(&num_binds),
    };

    let t = TestSetup::new_with_logic(logic).await;
    t.send_and_expect_response(
        b"\x00\x00\x00\x29\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        b"\x00\x00\x00\x1b\x80\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x06\
        TestServer\0",
    )
    .await;
    t.send_and_expect_response(
        b"\x00\x00\x00\x29\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        b"\x00\x00\x00\x1b\x80\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x06\
        TestServer\0",
    )
    .await;
    t.send_and_expect_response(
        b"\x00\x00\x00\x29\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x06\
        esmeid\0password\0type\0\x34\x00\x00\0",
        b"\x00\x00\x00\x1b\x80\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x06\
        TestServer\0",
    )
    .await;

    assert_eq!(*num_binds.lock().unwrap(), 3);
}

// TODO: receive MT (pluggable handler)
// TODO: return DR
// TODO: return MO
// Later: client app + system test that allows us to compare with CloudHopper
// Later: smpp session states (spec 2.2)
// Later: sc_interface_version TLV in bind response
// Later: Check interface versions in binds and responses, and submit_sm
// Later: all PDU types and formats
// Later: cargo features e.g. smsc, esme, pdu
