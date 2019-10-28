use std::time::Duration;
use quire::validate::{Structure, Sequence, Scalar, Numeric};

use crate::config::listen::{self, Listen};


#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Replication {
    pub listen: Listen,
    pub peers: Vec<String>,
    pub max_connections: usize,
    #[serde(with="::quire::duration")]
    pub listen_error_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub reconnect_timeout: Duration,
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
