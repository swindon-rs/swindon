use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::collections::HashMap;

use minihttp::client::{Codec, Config as HConfig, Proto};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tk_pool::Pool;
use tk_pool::uniform::{UniformMx, Config as PConfig};
use abstract_ns::{Router, Resolver, union_stream};
use futures::Stream;

use intern::Upstream;
use config::http_destinations::Destination;

pub type HttpPool = Pool<Box<Codec<TcpStream>+Send>>;

pub struct UpstreamRef<'a> {
    pools: &'a HttpPools,
    upstream: &'a Upstream,
}

pub struct UpstreamGuard<'a> {
    guard: RwLockWriteGuard<'a, HashMap<Upstream, HttpPool>>,
    upstream: &'a Upstream,
}

#[derive(Clone)]
pub struct HttpPools {
    plain: Arc<RwLock<HashMap<Upstream, HttpPool>>>,
    // TODO(tailhook) https pools
}

impl HttpPools {
    pub fn new() -> HttpPools {
        HttpPools {
            plain: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn upstream<'x>(&'x self, dest: &'x Upstream) -> UpstreamRef<'x> {
        UpstreamRef {
            pools: self,
            upstream: &dest,
        }
    }
    pub fn update(&self, cfg: &HashMap<Upstream, Destination>,
        resolver: &Router, handle: &Handle)
    {
        let mut plain = self.plain.write().expect("pools not poisoned");
        let mut to_delete = Vec::new();
        for k in plain.keys() {
            if !cfg.contains_key(k) {
                to_delete.push(k.clone());
            }
        }
        for k in to_delete {
            plain.remove(&k);
        }
        for (k, dest) in cfg {
            // TODO(tailhook) compare destinations
            if !plain.contains_key(k) {
                let h2 = handle.clone();
                let conn_config = HConfig::new()
                    .inflight_request_limit(
                        dest.in_flight_requests_per_backend_connection)
                    .done();
                let pool_config = PConfig::new()
                    .connections_per_address(
                        dest.backend_connections_per_ip_port)
                    .done();
                let stream = union_stream(
                    dest.addresses.iter()
                    .map(|x| resolver.subscribe(x)
                        as Box<Stream<Item=_, Error=_>>)
                    .collect());
                let mx = UniformMx::new(handle, &pool_config, Box::new(stream),
                    move |addr| Proto::connect_tcp(addr, &conn_config, &h2));
                let pool = Pool::create(handle, dest.queue_size_for_503, mx);
                plain.insert(k.clone(), pool);
            }
        }
     }
}

impl<'a> UpstreamRef<'a> {
    pub fn get_mut(&mut self) -> UpstreamGuard<'a> {
        UpstreamGuard {
            guard: self.pools.plain.write().expect("pools not poisoned"),
            upstream: self.upstream,
        }
    }
}

impl<'a> UpstreamGuard<'a> {
    pub fn get_mut(&mut self) -> Option<&mut HttpPool> {
        self.guard.get_mut(self.upstream)
    }
}
