use async_trait::async_trait;

use smpp::message_unique_key::MessageUniqueKey;
use smpp::pdu::{
    DeliverEsmClass, DeliverSmPdu, Pdu, SubmitEsmClass, SubmitSmPdu,
    SubmitSmRespPdu,
};
use smpp::smsc::{BindData, BindError, SmscLogic, SubmitSmError};

mod test_utils;

use test_utils::{TestClient, TestServer};

#[tokio::test]
async fn when_multiple_clients_send_mts_we_deliver_drs_to_the_right_one() {
    // 3 clients connect to a server
    let logic = Logic::new(vec![1, 2, 3]);
    let server = TestServer::start_with_logic_and_config(logic, 3)
        .await
        .unwrap();
    let mut client1 = TestClient::connect_to(&server).await.unwrap();
    let mut client2 = TestClient::connect_to(&server).await.unwrap();
    let mut client3 = TestClient::connect_to(&server).await.unwrap();

    client1.bind_transceiver().await;
    client2.bind_transceiver().await;
    client3.bind_transceiver().await;

    // Each client sends an MT
    client1
        .send_and_expect_response(&mt(1).await, &mt_resp(1).await)
        .await;
    client2
        .send_and_expect_response(&mt(2).await, &mt_resp(2).await)
        .await;
    client3
        .send_and_expect_response(&mt(3).await, &mt_resp(3).await)
        .await;

    // The DR for client3 comes back first
    server.receive_pdu(dr(3)).await.unwrap();
    // and it received it
    client3.expect_to_receive(&write(dr(3)).await).await;

    /* TODO: freezes
    // Then the others, and each goes to the client that sent the relevant MT
    server.receive_pdu(dr(1)).await.unwrap();
    server.receive_pdu(dr(2)).await.unwrap();
    // Reading in clients out-of-order is fine
    client2.expect_to_receive(&write(dr(2)).await).await;
    client1.expect_to_receive(&write(dr(1)).await).await;
    */
}

struct Logic {
    msgids: Vec<u32>,
}

impl Logic {
    fn new(mut msgids: Vec<u32>) -> Self {
        // We will pop ids off this, so reverse the order
        msgids.reverse();
        Self { msgids }
    }
}

#[async_trait]
impl SmscLogic for Logic {
    async fn bind(&mut self, _bind_data: &BindData) -> Result<(), BindError> {
        Ok(())
    }

    async fn submit_sm(
        &mut self,
        _pdu: &SubmitSmPdu,
    ) -> Result<(SubmitSmRespPdu, MessageUniqueKey), SubmitSmError> {
        let msgid = self
            .msgids
            .pop()
            .expect("Received more MTs than IDs I was given!");
        Ok((
            SubmitSmRespPdu::new(&msgid.to_string()).unwrap(),
            MessageUniqueKey::new(
                "multiclienttestsystem",
                &msgid.to_string(),
                "",
            ),
        ))
    }
}

fn dr(sequence_number: u32) -> Pdu {
    Pdu::new(
        0x00,
        sequence_number,
        DeliverSmPdu::new(
            "",
            0,
            0,
            "src_addr",
            0,
            0,
            "dest_addr",
            DeliverEsmClass::SmscDeliveryReceipt as u8,
            0x34,
            1,
            "",
            "",
            1,
            0,
            3,
            0,
            format!("id:{}", sequence_number).as_bytes(),
        )
        .unwrap()
        .into(),
    )
    .unwrap()
}

async fn mt(sequence_number: u32) -> Vec<u8> {
    let pdu: Pdu = Pdu::new(
        0,
        sequence_number,
        SubmitSmPdu::new(
            "",
            0,
            0,
            "src_addr",
            0,
            0,
            "dest_addr",
            SubmitEsmClass::Default as u8,
            0x34,
            1,
            "",
            "",
            1,
            0,
            3,
            0,
            b"dr \xffpls",
        )
        .unwrap()
        .into(),
    )
    .unwrap();

    write(pdu).await
}

async fn mt_resp(sequence_number: u32) -> Vec<u8> {
    let pdu: Pdu = Pdu::new(
        0,
        sequence_number,
        SubmitSmRespPdu::new(&sequence_number.to_string())
            .unwrap()
            .into(),
    )
    .unwrap();

    write(pdu).await
}

async fn write(pdu: Pdu) -> Vec<u8> {
    let mut ret: Vec<u8> = Vec::new();
    pdu.write(&mut ret).await.unwrap();
    ret
}

// TODO: deliver to the same client after they disconnect and reconnect
// TODO: drop DRs after some time trying to deliver
// TODO: multiple DRs to the same client
// TODO: send DR over a receiver connection when we bound as transmitter
