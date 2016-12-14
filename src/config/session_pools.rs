use std::sync::Arc;
use std::time::Duration;
use quire::De;
use quire::validate::{Structure, Sequence, Scalar};

use super::listen::{self, ListenSocket};
use super::http;

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct SessionPool {
    pub listen: ListenSocket,
    pub inactivity_handlers: Vec<http::Destination>,
    pub inactivity: Arc<InactivityTimeouts>,
}

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct InactivityTimeouts {
    pub new_connection: De<Duration>,
    pub client_min: De<Duration>,
    pub client_max: De<Duration>,
    pub client_default: De<Duration>,
}


pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("listen", listen::validator())
    .member("inactivity_handlers",
        Sequence::new(http::destination_validator()))
    .member("inactivity", Structure::new()
        .member("new_connection", Scalar::new().min_length(1).default("60s"))
        .member("client_min", Scalar::new().min_length(1).default("1s"))
        .member("client_max", Scalar::new().min_length(1).default("2h"))
        .member("client_default", Scalar::new().min_length(1).default("1s"))
    )
}
