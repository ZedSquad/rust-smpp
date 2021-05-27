use env_logger::Env;
use log::*;

use smpp::examples::smsc_drs_after_1_sec::DrsAfter1Sec;
use smpp::smsc;
use smpp::smsc::SmscConfig;

fn main() {
    let smsc_config = SmscConfig {
        bind_address: String::from("0.0.0.0:8080"),
        max_open_sockets: 100,
        system_id: String::from("rust_smpp"),
    };

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let res = smsc::run(smsc_config, DrsAfter1Sec::new());

    match res {
        Ok(_) => info!("Done"),
        Err(e) => error!("Error launching: {}", e),
    };
}
