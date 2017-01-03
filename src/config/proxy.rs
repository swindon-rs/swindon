use super::http;

use quire::validate::{Nothing, Enum, Structure, Scalar, Numeric};

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Mode {
    /// Means forward all headers including Host header
    forward,
}

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Proxy {
    pub mode: Mode,
    pub ip_header: Option<String>,
    pub destination: http::Destination,
    // TODO(tailhook) this might needs to be u64
    pub max_payload_size: usize,
    pub stream_requests: bool,
    pub response_buffer_size: usize,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("mode", Enum::new()
        .option("forward", Nothing)
        .allow_plain()
        .plain_default("forward"))
    .member("ip_header", Scalar::new().optional())
    .member("max_payload_size",
        Numeric::new().min(0).max(1 << 40).default(10 << 20))
    .member("stream_requests", Scalar::new().default(false))
    .member("response_buffer_size",
        Numeric::new().min(0).max(1 << 40).default(10 << 20))
    .member("destination", http::destination_validator())
}
