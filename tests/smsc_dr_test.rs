use async_trait::async_trait;

use smpp::pdu::{
    DeliverEsmClass, DeliverSmPdu, Pdu, SubmitSmPdu, SubmitSmRespPdu,
};
use smpp::smsc::{BindData, BindError, SmscLogic, SubmitSmError};

mod test_utils;

use test_utils::{bytes_as_string, TestSetup};

#[tokio::test]
async fn when_we_receive_deliver_sm_for_a_message_we_provide_it_to_client() {
    let msgid = "ab87J";
    let submit_sm = new_submit_sm(0x2f).await;
    let submit_sm_resp = new_submit_sm_resp(0x2f, msgid).await;
    let logic = Logic {
        msgid: String::from(msgid),
    };

    let t = TestSetup::new_with_logic(logic).await;
    let mut t = t.bind().await;

    t.send_and_expect_response(&submit_sm, &submit_sm_resp)
        .await;

    let deliver_sm_pdu = new_deliver_sm_pdu(b"id:8765");
    let mut deliver_sm = Vec::new();
    deliver_sm_pdu.write(&mut deliver_sm).await.unwrap();

    t.receive_pdu(deliver_sm_pdu).await.unwrap();

    let resp = t.client.read_n(deliver_sm.len()).await;
    assert_eq!(bytes_as_string(&resp), bytes_as_string(&deliver_sm));
}

struct Logic {
    msgid: String,
}

#[async_trait]
impl SmscLogic for Logic {
    async fn bind(&mut self, _bind_data: &BindData) -> Result<(), BindError> {
        Ok(())
    }

    async fn submit_sm(
        &mut self,
        _pdu: &SubmitSmPdu,
    ) -> Result<SubmitSmRespPdu, SubmitSmError> {
        Ok(SubmitSmRespPdu::new(&self.msgid).unwrap())
    }
}

fn new_deliver_sm_pdu(short_message: &[u8]) -> Pdu {
    Pdu::new(
        0x00,
        0x6d,
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
            short_message,
            // TODO: correct esm_class here and in submit_sm tests
            // TODO: check for correct esm class in parsing/smsc code?
            // TODO: more complete short_message and/or TLV receipted_message_id
        )
        .unwrap()
        .into(),
    )
    .unwrap()
}

async fn new_submit_sm(sequence_number: u32) -> Vec<u8> {
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
            0,
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

    let mut ret: Vec<u8> = Vec::new();
    pdu.write(&mut ret).await.unwrap();

    ret
}

async fn new_submit_sm_resp(sequence_number: u32, msgid: &str) -> Vec<u8> {
    let pdu: Pdu = Pdu::new(
        0,
        sequence_number,
        SubmitSmRespPdu::new(msgid).unwrap().into(),
    )
    .unwrap();

    let mut ret: Vec<u8> = Vec::new();
    pdu.write(&mut ret).await.unwrap();

    ret
}

// TODO: Client sends deliver_sm_resp and it comes through to SmscLogic
// TODO: multiple clients, and DRs go back to the right one
