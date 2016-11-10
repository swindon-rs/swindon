use std::net::SocketAddr;

use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Enum, Scalar};


#[derive(Debug, PartialEq, Eq)]
pub enum ListenSocket {
    Tcp(SocketAddr),
    // TODO(tailhook)
    // Fd(u32)
    // Unix(PathBuf)
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("Tcp", Scalar::new())
    .default_tag("Tcp")
}

impl Decodable for ListenSocket {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_str()?
        .parse()
        .map(ListenSocket::Tcp)
        .map_err(|_| d.error("Can't parse socket address"))
    }
}
