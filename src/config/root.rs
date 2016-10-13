//! Root config validator

use quire::validate::{Structure, Sequence};
use super::listen::{self, ListenSocket};
use super::routing::{self, Routing};

#[derive(RustcDecodable)]
pub struct Config {
    pub listen: Vec<ListenSocket>,
    pub routing: Routing,
}

pub fn config_validator<'a>() -> Structure<'a> {
    Structure::new()
    .member("listen", Sequence::new(listen::validator()))
    .member("routing", routing::validator())
}
