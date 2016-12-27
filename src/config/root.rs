//! Root config validator
use std::collections::HashMap;
use std::sync::Arc;

use quire::validate::{Structure, Sequence, Mapping, Scalar, Numeric};

use intern::{HandlerName, Upstream, SessionPoolName, DiskPoolName};
use super::listen::{self, ListenSocket};
use super::routing::{self, Routing};
use super::handlers::{self, Handler};
use super::session_pools::{self, SessionPool};
use super::http_destinations::{self, Destination};
use super::disk::{self, Disk};

#[derive(RustcDecodable, PartialEq, Eq, Debug)]
pub struct Config {
    pub listen: Vec<ListenSocket>,
    pub max_connections: usize,
    pub pipeline_depth: usize,
    pub routing: Routing,
    pub handlers: HashMap<HandlerName, Handler>,
    pub session_pools: HashMap<SessionPoolName, Arc<SessionPool>>,
    pub http_destinations: HashMap<Upstream, Destination>,
    pub debug_routing: bool,
    pub server_name: Option<String>,
    /// Note: "default" disk pool is always created, the only thing you can
    /// do is to update it's pool size, It's pool size can't be less than
    /// one, however.
    pub disk_pools: HashMap<DiskPoolName, Disk>,
}

pub fn config_validator<'a>() -> Structure<'a> {
    Structure::new()
    .member("listen", Sequence::new(listen::validator()))
    .member("max_connections",
        Numeric::new().min(1).max(1 << 31).default(1000))
    .member("pipeline_depth",
        Numeric::new().min(1).max(10000).default(2))
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
