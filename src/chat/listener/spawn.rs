use std::sync::Arc;

use futures::stream::Stream;
use tk_http;
use tk_http::server::Proto;
use tk_listen::{BindMany, ListenExt};
use futures::future::{Future};
use tokio_core::reactor::{Handle};
use ns_router::future::AddrStream;

use crate::intern::SessionPoolName;
use crate::config::SessionPool;
use crate::runtime::Runtime;
use crate::chat::listener::codec::Handler;
use crate::chat::processor::{ProcessorPool};
use crate::chat::replication::RemotePool;


pub struct WorkerData {
    pub name: SessionPoolName,
    pub runtime: Arc<Runtime>,
    pub settings: Arc<SessionPool>,
    pub processor: ProcessorPool,
    pub remote: RemotePool,

    pub handle: Handle, // Does it belong here?
}

pub fn listen(addr_stream: AddrStream, worker_data: &Arc<WorkerData>) {
    let w1 = worker_data.clone();
    let w2 = worker_data.clone();
    let runtime = worker_data.runtime.clone();
    let h1 = runtime.handle.clone();

    // TODO(tailhook) how to update?
    let hcfg = tk_http::server::Config::new()
        .inflight_request_limit(worker_data.settings.pipeline_depth)
        // TODO(tailhook) make it configurable?
        .inflight_request_prealoc(0)
        .done();

    worker_data.handle.spawn(
        BindMany::new(addr_stream.map(|addr| addr.addresses_at(0)), &h1)
        .sleep_on_error(w1.settings.listen_error_timeout, &runtime.handle)
        .map(move |(socket, saddr)| {
             Proto::new(socket, &hcfg, Handler::new(saddr, w2.clone()), &h1)
             .map_err(|e| debug!("Chat backend protocol error: {}", e))
        })
        .listen(worker_data.settings.max_connections)
        .map(move |()| error!("Replication listener exited"))
        .map_err(move |()| error!("Replication listener errored"))
    );
}
