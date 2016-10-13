//! Root config validator

use quire::validate::{Structure};

#[derive(RustcDecodable)]
pub struct Config {

}

pub fn config_validator<'a>() -> Structure<'a> {
    Structure::new()
}
