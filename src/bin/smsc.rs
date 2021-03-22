use async_trait::async_trait;
use env_logger::Env;
use log::*;

use smpp::message_unique_key::MessageUniqueKey;
use smpp::pdu::{SubmitSmPdu, SubmitSmRespPdu};
use smpp::smsc;
use smpp::smsc::SmscConfig;
use smpp::smsc::{BindData, BindError, SmscLogic, SubmitSmError};

fn main() {
    let smsc_config = SmscConfig {
        bind_address: String::from("0.0.0.0:8080"),
        max_open_sockets: 100,
        system_id: String::from("rust_smpp"),
    };

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    // Always consider all system_id/password combinations valid
    struct Logic {}

    #[async_trait]
    impl SmscLogic for Logic {
        async fn bind(
            &mut self,
            _bind_data: &BindData,
        ) -> Result<(), BindError> {
            Ok(())
        }

        async fn submit_sm(
            &mut self,
            _pdu: &SubmitSmPdu,
        ) -> Result<(SubmitSmRespPdu, MessageUniqueKey), SubmitSmError>
        {
            Err(SubmitSmError::InternalError)
        }
    }

    let res = smsc::run(smsc_config, Logic {});

    match res {
        Ok(_) => info!("Done"),
        Err(e) => error!("Error launching: {}", e),
    };
}
