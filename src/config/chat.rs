use std::collections::BTreeMap;

use quire::validate::{Structure, Scalar, Mapping};

use super::listen::{self, ListenSocket};
use super::http;


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Chat {
    pub listen: ListenSocket,
    pub http_route: http::Destination,
    pub message_handlers: BTreeMap<String, http::Destination>,
}


pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("listen", listen::validator())
    .member("http_route", http::destination_validator())
    .member("message_handlers",
        Mapping::new(Scalar::new(), http::destination_validator()))
}
