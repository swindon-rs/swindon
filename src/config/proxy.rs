use super::http;

use quire::validate::{Nothing, Enum, Structure, Scalar};

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Mode {
    /// Means forward all headers including Host header
    forward,
}

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Proxy {
    pub mode: Mode,
    pub ip_header: String,
    pub destination: http::Destination,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("mode", Enum::new()
        .option("forward", Nothing)
        .allow_plain())
    .member("ip_header", Scalar::new())
    .member("destination", http::destination_validator())
}
