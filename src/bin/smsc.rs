use env_logger::Env;
use log::*;
use std::sync::{Arc, Mutex};

use smpp::smsc_app;
use smpp::smsc_app::{BindData, BindError, SmscLogic};
use smpp::smsc_config::SmscConfig;

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
    impl SmscLogic for Logic {
        fn bind(&self, _bind_data: &BindData) -> Result<(), BindError> {
            Ok(())
        }
    }
    let logic = Arc::new(Mutex::new(Logic {}));

    let res = smsc_app::run(smsc_config, logic);

    match res {
        Ok(_) => info!("Done"),
        Err(e) => error!("Error launching: {}", e),
    };
}
