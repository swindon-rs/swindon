use std::fmt;
use std::net::SocketAddr;

use serde::de::{self, Deserialize, Deserializer};
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

struct Visitor;

impl<'a> de::Visitor<'a> for Visitor {
    type Value = ListenSocket;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("ip_address:port")
    }
    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        s.parse()
        .map(ListenSocket::Tcp)
        .map_err(|_| E::custom("Can't parse socket address"))
    }
}

impl<'a> Deserialize<'a> for ListenSocket {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(Visitor)
    }
}
