use quire::validate::{Structure, Sequence, Numeric};

use super::listen::{self, ListenSocket};
use super::http;

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct SessionPool {
    pub listen: ListenSocket,
    pub inactivity_handlers: Vec<http::Destination>,
    pub inactivity: InactivityTimeouts,
}

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct InactivityTimeouts {
    pub new_connection: u64,
    pub client_min: u64,
    pub client_max: u64,
    pub client_default: u64,
}


pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("listen", listen::validator())
    .member("inactivity_handlers",
        Sequence::new(http::destination_validator()))
    .member("inactivity", Structure::new()
        .member("new_connection", Numeric::new().min(0).default(3600))
        .member("client_min", Numeric::new().min(0).default(1))
        .member("client_max", Numeric::new().min(0).default(3600))
        .member("client_default", Numeric::new().min(0).default(30))
    )
}
