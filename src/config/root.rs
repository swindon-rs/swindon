//! Root config validator
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;

use quire::validate::{Structure, Sequence, Mapping, Scalar, Numeric};

use crate::intern::{HandlerName, Upstream, SessionPoolName, DiskPoolName};
use crate::intern::{LdapUpstream, Network, Authorizer as AuthorizerName};
use crate::intern::{LogFormatName};
use crate::config::listen::{self, Listen};
use crate::config::routing::{self, HostPath, RouteDef};
use crate::config::handlers::{self, Handler};
use crate::config::authorizers::{self, Authorizer};
use crate::config::session_pools::{self, SessionPool};
use crate::config::http_destinations::{self, Destination};
use crate::config::ldap;
use crate::config::log;
use crate::config::networks;
use crate::config::disk::{self, Disk};
use crate::config::replication::{self, Replication};
use crate::routing::RoutingTable;


#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct Mixin {
    pub handlers: HashMap<HandlerName, Handler>,
    pub authorizers: HashMap<AuthorizerName, Authorizer>,
    pub session_pools: HashMap<SessionPoolName, Arc<SessionPool>>,
    pub http_destinations: HashMap<Upstream, Arc<Destination>>,
    pub ldap_destinations: HashMap<LdapUpstream, ldap::Destination>,
    pub networks: HashMap<Network, networks::NetworkList>,
    pub log_formats: HashMap<LogFormatName, log::Format>,
    /// Note: "default" disk pool is always created, the only thing you can
    /// do is to update it's pool size, It's pool size can't be less than
    /// one, however.
    pub disk_pools: HashMap<DiskPoolName, Disk>,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct ConfigSource {
    pub listen: Listen,
    pub max_connections: usize,
    pub pipeline_depth: usize,
    #[serde(with="::quire::duration")]
    pub listen_error_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub first_byte_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub keep_alive_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub headers_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub input_body_byte_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub input_body_whole_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub output_body_byte_timeout: Duration,
    #[serde(with="::quire::duration")]
    pub output_body_whole_timeout: Duration,

    pub routing: HashMap<HostPath, RouteDef>,

    pub handlers: HashMap<HandlerName, Handler>,
    pub authorizers: HashMap<AuthorizerName, Authorizer>,
    pub session_pools: HashMap<SessionPoolName, Arc<SessionPool>>,
    pub http_destinations: HashMap<Upstream, Arc<Destination>>,
    pub ldap_destinations: HashMap<LdapUpstream, ldap::Destination>,
    pub networks: HashMap<Network, networks::NetworkList>,
    pub log_formats: HashMap<LogFormatName, log::Format>,
    /// Note: "default" disk pool is always created, the only thing you can
    /// do is to update it's pool size, It's pool size can't be less than
    /// one, however.
    pub disk_pools: HashMap<DiskPoolName, Disk>,

    pub replication: Arc<Replication>,
    pub debug_routing: bool,
    pub debug_logging: bool,
    pub server_name: Option<String>,

    pub set_user: Option<String>,
    pub set_group: Option<String>,

    /// We need to keep order of mixins stable for the purpose
    /// of fingerprinting
    pub mixins: BTreeMap<String, PathBuf>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ConfigData {
    pub listen: Listen,
    pub max_connections: usize,
    pub pipeline_depth: usize,
    pub listen_error_timeout: Duration,
    pub first_byte_timeout: Duration,
    pub keep_alive_timeout: Duration,
    pub headers_timeout: Duration,
    pub input_body_byte_timeout: Duration,
    pub input_body_whole_timeout: Duration,
    pub output_body_byte_timeout: Duration,
    pub output_body_whole_timeout: Duration,

    pub routing: RoutingTable,

    pub handlers: HashMap<HandlerName, Handler>,
    pub authorizers: HashMap<AuthorizerName, Authorizer>,
    pub session_pools: HashMap<SessionPoolName, Arc<SessionPool>>,
    pub http_destinations: HashMap<Upstream, Arc<Destination>>,
    pub ldap_destinations: HashMap<LdapUpstream, ldap::Destination>,
    pub networks: HashMap<Network, networks::NetworkList>,
    pub log_formats: HashMap<LogFormatName, log::Format>,
    pub disk_pools: HashMap<DiskPoolName, Disk>,

    pub replication: Arc<Replication>,
    pub debug_routing: bool,
    pub debug_logging: bool,
    pub server_name: Option<String>,

    pub set_user: Option<String>,
    pub set_group: Option<String>,
}

trait MixinSections {
    fn add_sections(self) -> Self;
}

impl<'a> MixinSections for Structure<'a> {
    fn add_sections(self) -> Self {
        self
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
        .member("disk_pools", Mapping::new(Scalar::new(), disk::validator()))
    }
}

pub fn mixin_validator<'a>() -> Structure<'a> {
    Structure::new()
    .add_sections()
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

    .member("replication", replication::validator())
    .member("debug_routing", Scalar::new().default(false))
    .member("debug_logging", Scalar::new().default(false))
    .member("server_name", Scalar::new().optional()
        .default(concat!("swindon/", env!("CARGO_PKG_VERSION"))))
    .member("set_user", Scalar::new().optional())
    .member("set_group", Scalar::new().optional())

    .member("mixins", Mapping::new(Scalar::new(), Scalar::new()))
    .add_sections()
}
