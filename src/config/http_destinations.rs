use std::time::Duration;

use quire::validate::{Structure, Scalar, Enum, Numeric, Nothing};
use quire::validate::{Sequence};
use quire::De;

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum LoadBalancing {
    queue,
}

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Destination {
    pub load_balancing: LoadBalancing,
    pub queue_size_for_503: usize,
    pub backend_connections_per_ip_port: usize,
    pub in_flight_requests_per_backend_connection: usize,
    pub addresses: Vec<String>,
    pub keep_alive_timeout: De<Duration>,
    pub max_request_timeout: De<Duration>,
    pub safe_pipelining_timeout: De<Duration>,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("load_balancing", Enum::new()
        .option("queue", Nothing)
        .allow_plain()
        .plain_default("queue"))
    .member("queue_size_for_503",
        Numeric::new().min(0).max(1 << 32).default(100_000))
    .member("backend_connections_per_ip_port",
        Numeric::new().min(1).max(100_000).default(100))
    .member("in_flight_requests_per_backend_connection",
        Numeric::new().min(1).max(1000).default(2))
    .member("addresses", Sequence::new(Scalar::new()).min_length(1))
    .member("keep_alive_timeout", Scalar::new().default("4 sec"))
    .member("max_request_timeout", Scalar::new().default("30 secs"))
    .member("safe_pipeline_timeout", Scalar::new().default("300 ms"))
}
