use crate::pdu::data::bind_data::BindData;
use crate::pdu::PduStatus;

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

pub trait SmscLogic {
    fn bind(&self, bind_data: &BindData) -> Result<(), BindError>;
}
