use ascii::AsciiString;

#[derive(Clone, Debug)]
pub struct SmscConfig {
    pub bind_address: String,
    pub max_open_sockets: usize,
    pub system_id: AsciiString,
}
