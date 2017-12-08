use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::collections::HashMap;
use std::net::SocketAddr;

use ns_router::{Router};
use tk_http::client::{Codec, Config as HConfig, Proto, Error, EncoderDone};
use tk_pool::metrics::Collect;
use tk_pool::config::{NewErrorLog, NewMetrics};
use tk_pool::error_log::{ShutdownReason, ErrorLog};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tk_pool::queue::Pool;
use tk_pool::pool_for;
use futures::future::FutureResult;
use libcantal::{Collection, Visitor};

use intern::Upstream;
use config::http_destinations::Destination;
use metrics::{Counter, List, Metric, Integer};

lazy_static! {
    pub static ref REQUESTS: Counter = Counter::new();
    pub static ref FAILED_503: Counter = Counter::new();

    pub static ref CONNECTING: Integer = Integer::new();
    pub static ref CONNECTED: Integer = Integer::new();
    pub static ref BLACKLISTED: Integer = Integer::new();
    pub static ref REQUEST_QUEUE: Integer = Integer::new();

    pub static ref CONNECTION_ATTEMPTED: Counter = Counter::new();
    pub static ref CONNECTION_ABORTED: Counter = Counter::new();
    pub static ref CONNECTION_ERRORED: Counter = Counter::new();
    pub static ref CONNECTION_ESTABLISHED: Counter = Counter::new();
    pub static ref CONNECTION_DROPPED: Counter = Counter::new();
    pub static ref BLACKLIST_ADDED: Counter = Counter::new();
    pub static ref BLACKLIST_REMOVED: Counter = Counter::new();
    pub static ref REQUESTS_QUEUED: Counter = Counter::new();
    pub static ref REQUESTS_FORWARDED: Counter = Counter::new();

    pub static ref POOLS: Integer = Integer::new();
    pub static ref POOLS_STARTED: Counter = Counter::new();
    pub static ref POOLS_STOPPED: Counter = Counter::new();
}

/// Future that is used for sending a client request
///
/// While we only support fully buffered requests it's fine to use
/// FutureResult, but we will probably change it to something
pub type HttpFuture<S> = FutureResult<EncoderDone<S>, Error>;
pub type PoolInner = Pool<
    Box<Codec<TcpStream, Future=HttpFuture<TcpStream>>+Send>,
    PoolMetrics>;

pub struct HttpPool {
    pool: PoolInner,
    metrics: PoolMetrics,
}

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

#[derive(Clone, Debug)]
pub struct PoolMetrics(Arc<Metrics>);

#[derive(Clone, Debug)]
pub struct PoolLog(Upstream);

#[derive(Debug)]
struct Metrics {
    name: Upstream,

    connecting: Integer,
    connected: Integer,
    blacklisted: Integer,
    request_queue: Integer,

    connection_attempted: Counter,
    connection_aborted: Counter,
    connection_errored: Counter,
    connection_established: Counter,
    connection_dropped: Counter,
    blacklist_added: Counter,
    blacklist_removed: Counter,
    requests_queued: Counter,
    requests_forwarded: Counter,
}

impl Metrics {
    fn new(name: &Upstream) -> Metrics {
        POOLS.incr(1);
        POOLS_STARTED.incr(1);
        Metrics {
            name: name.clone(),

            connecting: Integer::new(),
            connected: Integer::new(),
            blacklisted: Integer::new(),
            request_queue: Integer::new(),

            connection_attempted: Counter::new(),
            connection_aborted: Counter::new(),
            connection_errored: Counter::new(),
            connection_established: Counter::new(),
            connection_dropped: Counter::new(),
            blacklist_added: Counter::new(),
            blacklist_removed: Counter::new(),
            requests_queued: Counter::new(),
            requests_forwarded: Counter::new(),
        }
    }
}

impl Collection for PoolMetrics {
    fn visit<'x>(&'x self, v: &mut Visitor<'x>) {
        use metrics::Metric as M;
        let ref s = self.0;
        let g = format!("http.pools.{}", s.name);
        v.metric(&M(&g, "connecting"), &s.connecting);
        v.metric(&M(&g, "connecting"), &s.connecting);
        v.metric(&M(&g, "connected"), &s.connected);
        v.metric(&M(&g, "blacklisted"), &s.blacklisted);
        v.metric(&M(&g, "request_queue"), &s.request_queue);

        v.metric(&M(&g, "connection_attempted"), &s.connection_attempted);
        v.metric(&M(&g, "connection_aborted"), &s.connection_aborted);
        v.metric(&M(&g, "connection_errored"), &s.connection_errored);
        v.metric(&M(&g, "connection_established"), &s.connection_established);
        v.metric(&M(&g, "connection_dropped"), &s.connection_dropped);
        v.metric(&M(&g, "blacklist_added"), &s.blacklist_added);
        v.metric(&M(&g, "blacklist_removed"), &s.blacklist_removed);
        v.metric(&M(&g, "requests_queued"), &s.requests_queued);
        v.metric(&M(&g, "requests_forwarded"), &s.requests_forwarded);
    }
}

impl PoolMetrics {
    fn new(name: &Upstream) -> PoolMetrics {
        PoolMetrics(Arc::new(Metrics::new(name)))
    }
}

impl Collect for PoolMetrics {
    fn connection_attempt(&self) {
        CONNECTING.incr(1);
        self.0.connecting.incr(1);
        CONNECTION_ATTEMPTED.incr(1);
        self.0.connection_attempted.incr(1);
    }
    fn connection_abort(&self) {
        CONNECTING.decr(1);
        self.0.connecting.decr(1);
        CONNECTION_ABORTED.incr(1);
        self.0.connection_aborted.incr(1);
    }
    fn connection_error(&self) {
        CONNECTING.decr(1);
        self.0.connecting.decr(1);
        CONNECTION_ERRORED.incr(1);
        self.0.connection_errored.incr(1);
    }
    fn connection(&self) {
        CONNECTING.decr(1);
        self.0.connecting.decr(1);
        CONNECTION_ESTABLISHED.incr(1);
        self.0.connection_established.incr(1);
        CONNECTED.incr(1);
        self.0.connected.incr(1);
    }
    fn disconnect(&self) {
        CONNECTED.decr(1);
        self.0.connected.decr(1);
        CONNECTION_DROPPED.incr(1);
        self.0.connection_dropped.incr(1);
    }
    fn blacklist_add(&self) {
        BLACKLISTED.incr(1);
        self.0.blacklisted.incr(1);
        BLACKLIST_ADDED.incr(1);
        self.0.blacklist_added.incr(1);
    }
    fn blacklist_remove(&self) {
        BLACKLISTED.decr(1);
        self.0.blacklisted.decr(1);
        BLACKLIST_REMOVED.incr(1);
        self.0.blacklist_removed.incr(1);
    }
    fn request_queued(&self) {
        REQUEST_QUEUE.incr(1);
        self.0.request_queue.incr(1);
        REQUESTS_QUEUED.incr(1);
        self.0.requests_queued.incr(1);
    }
    fn request_forwarded(&self) {
        REQUEST_QUEUE.decr(1);
        self.0.request_queue.decr(1);
        REQUESTS_FORWARDED.incr(1);
        self.0.requests_forwarded.incr(1);
    }
    fn pool_closed(&self) {
        // TODO(tailhook) decrement global counters?
        POOLS.decr(1);
        POOLS_STOPPED.incr(1);
    }
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
                let metrics = PoolMetrics::new(k);
                let pool = pool_for(move |addr| {
                        Proto::connect_tcp(addr, &conn_config, &h2)
                    })
                    .connect_to(resolver.subscribe_many(&dest.addresses, 80))
                    .lazy_uniform_connections(
                        dest.backend_connections_per_ip_port as u32)
                    .with_queue_size(
                        dest.queue_size_for_503)
                    .metrics(metrics.clone())
                    .errors(PoolLog(k.clone()))
                    .spawn_on(handle);
                plain.insert(k.clone(), HttpPool { pool, metrics });
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
    pub fn get_mut(&mut self) -> Option<&mut PoolInner> {
        self.guard.get_mut(self.upstream).map(|x| &mut x.pool)
    }
}

pub fn metrics() -> List {
    let base = "http.outgoing";
    vec![
        // obeys cantal-py.RequestTracker
        (Metric(base, "requests"), &*REQUESTS),
        (Metric(base, "backpressure_failures"), &*FAILED_503),

        (Metric(base, "connecting"), &*CONNECTING),
        (Metric(base, "connected"), &*CONNECTED),
        (Metric(base, "blacklisted"), &*BLACKLISTED),
        (Metric(base, "request_queue"), &*REQUEST_QUEUE),

        (Metric(base, "connection_attempted"), &*CONNECTION_ATTEMPTED),
        (Metric(base, "connection_aborted"), &*CONNECTION_ABORTED),
        (Metric(base, "connection_errored"), &*CONNECTION_ERRORED),
        (Metric(base, "connection_established"), &*CONNECTION_ESTABLISHED),
        (Metric(base, "connection_dropped"), &*CONNECTION_DROPPED),
        (Metric(base, "blacklist_added"), &*BLACKLIST_ADDED),
        (Metric(base, "blacklist_removed"), &*BLACKLIST_REMOVED),
        (Metric(base, "requests_queued"), &*REQUESTS_QUEUED),
        (Metric(base, "requests_forwarded"), &*REQUESTS_FORWARDED),

        (Metric(base, "pools"), &*POOLS),
        (Metric(base, "pools_started"), &*POOLS_STARTED),
        (Metric(base, "pools_stopped"), &*POOLS_STOPPED),
    ]
}

pub fn pool_metrics(h: &HttpPools) -> Vec<PoolMetrics> {
    h.plain.read().expect("http pools are okay")
        .values()
        .map(|p| p.metrics.clone())
        .collect()
}


impl NewErrorLog<Error, Error> for PoolLog {
    type ErrorLog = PoolLog;
    fn construct(self) -> Self::ErrorLog {
        self
    }
}

impl NewMetrics for PoolMetrics {
    type Collect = PoolMetrics;
    fn construct(self) -> Self::Collect {
        self.clone()
    }
}

impl ErrorLog for PoolLog {
    type ConnectionError = Error;
    type SinkError = Error;
    fn connection_error(&self, addr: SocketAddr, e: Self::ConnectionError) {
        warn!("{}: Connecting to {} failed: {}", self.0, addr, e);
    }
    fn sink_error(&self, addr: SocketAddr, e: Self::SinkError) {
        warn!("{}: Connection to {} errored: {}", self.0, addr, e);
    }
    /// Starting to shut down pool
    fn pool_shutting_down(&self, reason: ShutdownReason) {
        warn!("{}: Shutting down connection pool: {}", self.0, reason);
    }
    /// This is triggered when pool done all the work and shut down entirely
    fn pool_closed(&self) {
        info!("{}: Pool closed", self.0);
    }
}
