//! Root config validator

use quire::validate::{Structure, Sequence};
use super::listen::{self, ListenSocket};

#[derive(RustcDecodable)]
pub struct Config {
    pub listen: Vec<ListenSocket>,
}

pub fn config_validator<'a>() -> Structure<'a> {
    Structure::new()
    .member("listen", Sequence::new(listen::validator()))
}
