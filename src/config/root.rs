//! Root config validator
use std::collections::HashMap;

use quire::validate::{Structure, Sequence, Mapping, Scalar};

use super::listen::{self, ListenSocket};
use super::routing::{self, Routing};
use super::handlers::{self, Handler};
use super::http_destinations::{self, Destination};

#[derive(RustcDecodable, PartialEq, Eq, Debug)]
pub struct Config {
    pub listen: Vec<ListenSocket>,
    pub routing: Routing,
    pub handlers: HashMap<String, Handler>,
    pub http_destinations: HashMap<String, Destination>,
    pub debug_routing: bool,
}

pub fn config_validator<'a>() -> Structure<'a> {
    Structure::new()
    .member("listen", Sequence::new(listen::validator()))
    .member("routing", routing::validator())
    .member("handlers", Mapping::new(Scalar::new(), handlers::validator()))
    .member("http_destinations",
        Mapping::new(Scalar::new(), http_destinations::validator()))
    .member("debug_routing", Scalar::new().default(false))
}
