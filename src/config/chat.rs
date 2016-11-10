use std::collections::BTreeMap;

use quire::validate::{Structure, Scalar, Mapping};

use super::http;
use intern::Atom;


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Chat {
    pub session_pool: Atom,
    pub http_route: http::Destination,
    pub message_handlers: BTreeMap<String, http::Destination>,
}


pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("session_pool", Scalar::new())
    .member("http_route", http::destination_validator())
    .member("message_handlers",
        Mapping::new(Scalar::new(), http::destination_validator()))
}
