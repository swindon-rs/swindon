//! Root config validator
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use quire::validate::{Structure, Sequence, Mapping, Scalar, Numeric};
use quire::De;

use intern::{HandlerName, Upstream, SessionPoolName, DiskPoolName};
use intern::{LdapUpstream, Network, Authorizer as AuthorizerName};
use intern::{LogFormatName};
use config::listen::{self, ListenSocket};
use config::routing::{self, Routing};
use config::authorization::{self, Authorization};
use config::handlers::{self, Handler};
use config::authorizers::{self, Authorizer};
use config::session_pools::{self, SessionPool};
use config::http_destinations::{self, Destination};
use config::ldap;
use config::log;
use config::networks;
use config::disk::{self, Disk};
use super::replication::{self, Replication};

#[derive(RustcDecodable, PartialEq, Eq, Debug)]
pub struct ConfigData {
    pub listen: Vec<ListenSocket>,
    pub max_connections: usize,
    pub pipeline_depth: usize,
    pub listen_error_timeout: De<Duration>,
    pub first_byte_timeout: De<Duration>,
    pub keep_alive_timeout: De<Duration>,
    pub headers_timeout: De<Duration>,
    pub input_body_byte_timeout: De<Duration>,
    pub input_body_whole_timeout: De<Duration>,
    pub output_body_byte_timeout: De<Duration>,
    pub output_body_whole_timeout: De<Duration>,

    pub routing: Routing,
    pub authorization: Authorization,

    pub handlers: HashMap<HandlerName, Handler>,
    pub authorizers: HashMap<AuthorizerName, Authorizer>,
    pub session_pools: HashMap<SessionPoolName, Arc<SessionPool>>,
    pub http_destinations: HashMap<Upstream, Arc<Destination>>,
    pub ldap_destinations: HashMap<LdapUpstream, ldap::Destination>,
    pub networks: HashMap<Network, networks::NetworkList>,
    pub log_formats: HashMap<LogFormatName, log::Format>,

    pub replication: Arc<Replication>,
    pub debug_routing: bool,
    pub debug_logging: bool,
    pub server_name: Option<String>,

    pub set_user: Option<String>,
    pub set_group: Option<String>,

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
    .member("listen_error_timeout", Scalar::new().default("100ms"))
    .member("first_byte_timeout", Scalar::new().default("5s"))
    .member("keep_alive_timeout", Scalar::new().default("90s"))
    .member("headers_timeout", Scalar::new().default("10s"))
    .member("input_body_byte_timeout", Scalar::new().default("15s"))
    .member("input_body_whole_timeout", Scalar::new().default("1 hour"))
    .member("output_body_byte_timeout", Scalar::new().default("15s"))
    .member("output_body_whole_timeout", Scalar::new().default("1 hour"))

    .member("routing", routing::validator())
    .member("authorization", authorization::validator())

    .member("handlers", Mapping::new(Scalar::new(), handlers::validator()))
    .member("authorizers",
        Mapping::new(Scalar::new(), authorizers::validator()))
    .member("session_pools",
        Mapping::new(Scalar::new(), session_pools::validator()))
    .member("http_destinations",
        Mapping::new(Scalar::new(), http_destinations::validator()))
    .member("ldap_destinations",
        Mapping::new(Scalar::new(), ldap::destination_validator()))
    .member("networks", Mapping::new(Scalar::new(), networks::validator()))
    .member("log_formats", Mapping::new(Scalar::new(),
        log::format_validator()))

    .member("replication", replication::validator())
    .member("debug_routing", Scalar::new().default(false))
    .member("debug_logging", Scalar::new().default(false))
    .member("server_name", Scalar::new().optional()
        .default(concat!("swindon/", env!("CARGO_PKG_VERSION"))))
    .member("set_user", Scalar::new().optional())
    .member("set_group", Scalar::new().optional())
    .member("disk_pools", Mapping::new(Scalar::new(), disk::validator()))
}
