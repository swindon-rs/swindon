use std::sync::Arc;
use std::time::Duration;
use quire::De;
use quire::validate::{Structure, Sequence, Scalar, Numeric};

use super::listen::{self, ListenSocket};
use super::http;

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct SessionPool {
    pub listen: Vec<ListenSocket>,
    pub max_connections: usize,
    pub pipeline_depth: usize,
    pub listen_error_timeout: De<Duration>,
    pub max_payload_size: usize,
    pub inactivity_handlers: Vec<http::Destination>,
    pub new_connection_idle_timeout: De<Duration>,
    pub client_min_idle_timeout: De<Duration>,
    pub client_max_idle_timeout: De<Duration>,
    // XXX: never used
    pub client_default_idle_timeout: De<Duration>,
}


pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("listen", Sequence::new(listen::validator()))
    .member("pipeline_depth",
        Numeric::new().min(1).max(10000).default(2))
    .member("max_connections",
        Numeric::new().min(1).max(1 << 31).default(1000))
    .member("listen_error_timeout", Scalar::new().default("100ms"))
    .member("max_payload_size",
        Numeric::new().min(1).max(1 << 31).default(10_485_760))
    .member("inactivity_handlers",
        Sequence::new(http::destination_validator()))
    .member("new_connection_idle_timeout",
        Scalar::new().min_length(1).default("60s"))
    .member("client_min_idle_timeout",
        Scalar::new().min_length(1).default("1s"))
    .member("client_max_idle_timeout",
        Scalar::new().min_length(1).default("2h"))
    .member("client_default_idle_timeout",
        Scalar::new().min_length(1).default("1s"))
}
