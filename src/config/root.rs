//! Root config validator
use std::collections::HashMap;

use quire::validate::{Structure, Sequence, Mapping, Scalar};

use intern::Atom;
use super::listen::{self, ListenSocket};
use super::routing::{self, Routing};
use super::handlers::{self, Handler};
use super::session_pools::{self, Session};
use super::http_destinations::{self, Destination};
use super::disk::{self, Disk};

#[derive(RustcDecodable, PartialEq, Eq, Debug)]
pub struct Config {
    pub listen: Vec<ListenSocket>,
    pub routing: Routing,
    pub handlers: HashMap<Atom, Handler>,
    pub session_pools: HashMap<Atom, Session>,
    pub http_destinations: HashMap<Atom, Destination>,
    pub debug_routing: bool,
    pub server_name: Option<String>,
    /// Note: "default" disk pool is always created, the only thing you can
    /// do is to update it's pool size, It's pool size can't be less than
    /// one, however.
    pub disk_pools: HashMap<Atom, Disk>,
}

pub fn config_validator<'a>() -> Structure<'a> {
    Structure::new()
    .member("listen", Sequence::new(listen::validator()))
    .member("routing", routing::validator())
    .member("handlers", Mapping::new(Scalar::new(), handlers::validator()))
    .member("session_pools",
        Mapping::new(Scalar::new(), session_pools::validator()))
    .member("http_destinations",
        Mapping::new(Scalar::new(), http_destinations::validator()))
    .member("debug_routing", Scalar::new().default(false))
    .member("server_name", Scalar::new().optional()
        .default(concat!("swindon/", env!("CARGO_PKG_VERSION"))))
    .member("disk_pools", Mapping::new(Scalar::new(), disk::validator()))
}
