use std::sync::Arc;
use std::time::Duration;

use abstract_ns::HostResolve;
use async_slot as slot;
use futures::Stream;
use futures::future::{Future};
use futures_cpupool;
use ns_router::{self, SubscribeExt};
use ns_router::future::AddrStream;
use ns_std_threaded;
use self_meter_http::Meter;
use tk_http::server::Proto;
use tk_http;
use tk_listen::{BindMany, ListenExt};
use tokio_core::reactor::{Handle};
use void::Void;

use config::listen::Listen;
use config::{ConfigCell};
use incoming::Router;
use chat;
use runtime::Runtime;
use http_pools::{HttpPools};
use handlers::files::{DiskPools};
use request_id;


pub struct State {
    http_pools: HttpPools,
    session_pools: chat::SessionPools,
    disk_pools: DiskPools,
    listener_channel: slot::Sender<Listen>,
    replication_session: chat::ReplicationSession,
    runtime: Arc<Runtime>,
}

pub fn spawn_listener(addr_stream: AddrStream, handle: &Handle,
    runtime: &Arc<Runtime>, verbose: bool)
{
    let root = runtime.config.get();
    let r1 = runtime.clone();
    let r2 = runtime.clone();
    // TODO(tailhook) how to update?
    let hcfg = tk_http::server::Config::new()
        .inflight_request_limit(root.pipeline_depth)
        // TODO(tailhook) make it configurable?
        .inflight_request_prealoc(0)
        .first_byte_timeout(root.first_byte_timeout)
        .keep_alive_timeout(root.keep_alive_timeout)
        .headers_timeout(root.headers_timeout)
        .input_body_byte_timeout(root.input_body_byte_timeout)
        .input_body_whole_timeout(root.input_body_whole_timeout)
        .output_body_byte_timeout(root.output_body_byte_timeout)
        .output_body_whole_timeout(root.output_body_whole_timeout)
        .done();
    let h1 = handle.clone();
    handle.spawn(
        BindMany::new(addr_stream.map(move |addr| {
                if verbose {
                    println!("Listening at {}",
                        addr.addresses_at(0)
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>().join(", "));
                }
                addr.addresses_at(0)
            }), handle)
        .sleep_on_error(r1.config.get().listen_error_timeout, &r1.handle)
        .map(move |(socket, saddr)| {
             Proto::new(socket, &hcfg,
                Router::new(saddr, r2.clone(), h1.clone()), &h1)
             .map_err(|e| debug!("Http protocol error: {}", e))
        })
        .listen(root.max_connections)
        .map(move |()| panic!("Main listener exited"))
        .map_err(move |()| panic!("Main listener errored"))
    );
}

pub fn populate_loop(handle: &Handle, cfg: &ConfigCell, verbose: bool)
    -> State
{
    let meter = Meter::new();
    meter.spawn_scanner(handle);
    meter.track_current_thread_by_name();

    let ns_pool = {
        let m1 = meter.clone();
        let m2 = meter.clone();
        futures_cpupool::Builder::new()
        // TODO(tailhook) configure it
        .pool_size(5)
        .name_prefix("ns-resolver-")
        .after_start(move || m1.track_current_thread_by_name())
        .before_stop(move || m2.untrack_current_thread())
        .create()
    };

    let std = ns_std_threaded::ThreadedResolver::use_pool(ns_pool);

    // TODO(tailhook) add config for it, allow update
    let resolver = ns_router::Router::from_config(
        &ns_router::Config::new()
        .set_fallthrough(std.null_service_resolver()
            // TODO(tailhook) allow configure interval
            .interval_subscriber(Duration::new(1, 0), handle))
        .done(),
        handle);

    let server_id = request_id::new();
    let http_pools = HttpPools::new();
    let processor = chat::Processor::new();
    let mut replication_session = chat::ReplicationSession::new(
        processor.clone(), &resolver, handle, &server_id,
        &cfg.get().replication);
    let session_pools = chat::SessionPools::new(
        processor, replication_session.remote_sender.clone());
    let disk_pools = DiskPools::new(&meter);
    let runtime = Arc::new(Runtime {
        config: cfg.clone(),
        handle: handle.clone(),
        http_pools: http_pools.clone(),
        session_pools: session_pools.clone(),
        disk_pools: disk_pools.clone(),
        meter: meter,
        server_id: server_id,
        resolver: resolver.clone(),
    });
    let root = cfg.get();

    warn!("Started with server_id {}, config {}",
        server_id, cfg.fingerprint());

    let (listen_tx, listen_rx) = slot::channel();
    listen_tx.swap(root.listen.clone()).unwrap();

    spawn_listener(
        resolver.subscribe_stream(
            listen_rx.map_err(|()| -> Void { unreachable!() }), 80),
        handle, &runtime, verbose);

    disk_pools.update(&root.disk_pools);
    http_pools.update(&root.http_destinations, &resolver, handle);
    session_pools.update(&root.session_pools, handle, &runtime);
    replication_session.update(&cfg.get().replication, handle, &runtime);
    State {
        http_pools: http_pools,
        session_pools: session_pools,
        replication_session: replication_session,
        listener_channel: listen_tx,
        runtime: runtime,
        disk_pools: disk_pools,
    }
}

#[allow(dead_code)]
pub fn update_loop(state: &mut State, cfg: &ConfigCell, handle: &Handle) {
    state.listener_channel.swap(cfg.get().listen.clone())
        .map_err(|_| error!("Can't update listening sockets")).ok();
    state.disk_pools.update(&cfg.get().disk_pools);
    state.http_pools.update(&cfg.get().http_destinations,
        &state.runtime.resolver, handle);
    state.session_pools.update(&cfg.get().session_pools,
        handle, &state.runtime);
    state.replication_session.update(&cfg.get().replication,
        handle, &state.runtime);
}
