use ascii::AsciiString;
use env_logger::Env;
use log::*;

use smpp::smsc_app;
use smpp::smsc_config::SmscConfig;

fn main() {
    let smsc_config = SmscConfig {
        bind_address: String::from("0.0.0.0:8080"),
        max_open_sockets: 100,
        system_id: AsciiString::from_ascii("rust_smpp").unwrap(),
    };

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let res = smsc_app::run(smsc_config);

    match res {
        Ok(_) => info!("Done"),
        Err(e) => error!("Error launching: {}", e),
    };
}
