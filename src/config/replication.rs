use std::time::Duration;
use quire::validate::{Structure, Sequence, Scalar, Numeric};
use quire::De;

use super::listen::{self, ListenSocket};


#[derive(Debug, RustcDecodable, PartialEq, Eq)]
pub struct Replication {
    pub listen: Vec<ListenSocket>,
    pub peers: Vec<ListenSocket>,
    pub max_connections: usize,
    pub listen_error_timeout: De<Duration>,
    pub reconnect_timeout: De<Duration>,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("listen", Sequence::new(listen::validator()))
    .member("peers", Sequence::new(listen::validator()))
    .member("max_connections",
        Numeric::new().min(1).max(1 << 31).default(10))
    .member("listen_error_timeout", Scalar::new().default("100ms"))
    .member("reconnect_timeout", Scalar::new().default("5s"))
}
