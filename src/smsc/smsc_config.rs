use clap::Clap;

/// Short Message Service Center (SMSC) in Rust
#[derive(Clap, Clone, Debug)]
#[clap(name = "smsc")]
pub struct SmscConfig {
    /// Address to bind on
    #[clap(short, long, default_value = "0.0.0.0:8080")]
    pub bind_address: String,

    /// Maximum number of sockets that can be open
    #[clap(short, long, default_value = "100")]
    pub max_open_sockets: usize,

    /// system_id used as an identifier of the SMSC
    #[clap(short, long, default_value = "rust_smpp")]
    pub system_id: String,
}
