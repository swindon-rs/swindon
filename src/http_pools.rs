use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::collections::HashMap;

use ns_router::{Router};
use tk_http::client::{Codec, Config as HConfig, Proto, Error, EncoderDone};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tk_pool::Pool;
use tk_pool::uniform::{UniformMx, Config as PConfig};
use futures::future::FutureResult;

use intern::Upstream;
use config::http_destinations::Destination;
use metrics::{Counter, List, Metric};

lazy_static! {
    pub static ref REQUESTS: Counter = Counter::new();
    pub static ref FAILED_503: Counter = Counter::new();
}

/// Future that is used for sending a client request
///
/// While we only support fully buffered requests it's fine to use
/// FutureResult, but we will probably change it to something
pub type HttpFuture<S> = FutureResult<EncoderDone<S>, Error>;
pub type HttpPool = Pool<
        Box<Codec<TcpStream, Future=HttpFuture<TcpStream>>+Send>
    >;

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
    pub fn update(&self, cfg: &HashMap<Upstream, Arc<Destination>>,
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
                    .keep_alive_timeout(dest.keep_alive_timeout)
                    .safe_pipeline_timeout(dest.safe_pipeline_timeout)
                    .max_request_timeout(dest.max_request_timeout)
                    .done();
                let pool_config = PConfig::new()
                    .connections_per_address(
                        dest.backend_connections_per_ip_port)
                    .done();
                let addr = resolver.subscribe_many(&dest.addresses, 80);
                let mx = UniformMx::new(handle, &pool_config, addr,
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

pub fn metrics() -> List {
    vec![
        // obeys cantal-py.RequestTracker
        (Metric("http.outgoing", "requests"), &*REQUESTS),
        (Metric("http.outgoing", "backpressure_failures"), &*FAILED_503),
    ]
}
