use async_trait::async_trait;

use crate::message_unique_key::MessageUniqueKey;
use crate::pdu::data::bind_data::BindData;
use crate::pdu::PduStatus;
use crate::pdu::{SubmitSmPdu, SubmitSmRespPdu};

pub enum BindError {
    IncorrectPassword,
    InternalError,
}

impl From<BindError> for PduStatus {
    fn from(e: BindError) -> PduStatus {
        match e {
            BindError::IncorrectPassword => PduStatus::ESME_RINVPASWD,
            BindError::InternalError => PduStatus::ESME_RSYSERR,
        }
    }
}

pub enum SubmitSmError {
    InternalError,
}

impl From<SubmitSmError> for PduStatus {
    fn from(e: SubmitSmError) -> PduStatus {
        match e {
            SubmitSmError::InternalError => PduStatus::ESME_RSYSERR,
        }
    }
}

#[async_trait]
pub trait SmscLogic {
    async fn bind(&mut self, bind_data: &BindData) -> Result<(), BindError>;
    async fn submit_sm(
        &mut self,
        pdu: &SubmitSmPdu,
    ) -> Result<(SubmitSmRespPdu, MessageUniqueKey), SubmitSmError>;
}
