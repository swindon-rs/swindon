use quire::validate::{Structure, Sequence};

use super::listen::{self, ListenSocket};
use super::http;

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct SessionPool {
    pub listen: ListenSocket,
    pub inactivity_handlers: Vec<http::Destination>,
}


pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("listen", listen::validator())
    .member("inactivity_handlers",
        Sequence::new(http::destination_validator()))
}
