use std::io;
use std::sync::Arc;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::thread;

use abstract_ns;
use ns_std_threaded;
use futures::future::Either;
use tokio_core::reactor::{Handle, Timeout};
use tokio_core::net::TcpListener;
use futures::Stream;
use futures::future::{Future, ok};
use futures::sync::oneshot::{channel as oneshot, Sender, Receiver};
use futures_cpupool;
use self_meter_http::Meter;
use tk_http;
use tk_http::server::Proto;
use tk_listen::ListenExt;

use config::{ListenSocket, ConfigCell};
use incoming::Router;
use chat;
use handlers;
use runtime::Runtime;
use http_pools::{HttpPools};


pub struct State {
    http_pools: HttpPools,
    session_pools: chat::SessionPools,
    ns: abstract_ns::Router,
    listener_shutters: HashMap<SocketAddr, Sender<()>>,
    runtime: Arc<Runtime>,
}

pub fn spawn_listener(addr: SocketAddr, handle: &Handle,
    runtime: &Arc<Runtime>, shutter: Receiver<()>)
    -> Result<(), io::Error>
{
    let root = runtime.config.get();
    let r1 = runtime.clone();
    let r2 = runtime.clone();
    let listener = TcpListener::bind(&addr, &handle)?;
    // TODO(tailhook) how to update?
    let hcfg = tk_http::server::Config::new()
        .inflight_request_limit(root.pipeline_depth)
        // TODO(tailhook) make it configurable?
        .inflight_request_prealoc(0)
        .first_byte_timeout(*root.first_byte_timeout)
        .keep_alive_timeout(*root.keep_alive_timeout)
        .headers_timeout(*root.headers_timeout)
        .input_body_byte_timeout(*root.input_body_byte_timeout)
        .input_body_whole_timeout(*root.input_body_whole_timeout)
        .output_body_byte_timeout(*root.output_body_byte_timeout)
        .output_body_whole_timeout(*root.output_body_whole_timeout)
        .done();
    let h1 = handle.clone();

    handle.spawn(
        listener.incoming()
        .sleep_on_error(*r1.config.get().listen_error_timeout, &r1.handle)
        .map(move |(socket, saddr)| {
             Proto::new(socket, &hcfg,
                Router::new(saddr, r2.clone(), h1.clone()), &h1)
             .map_err(|e| debug!("Http protocol error: {}", e))
        })
        .listen(root.max_connections)
        .select(shutter.map_err(|_| unreachable!()))
        .map(move |(_, _)| info!("Listener {} exited", addr))
        .map_err(move |(_, _)| info!("Listener {} exited", addr))
    );
    Ok(())
}

pub fn populate_loop(handle: &Handle, cfg: &ConfigCell, verbose: bool)
    -> State
{
    let mut meter = Meter::new();
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

    let ns = ns_std_threaded::ThreadedResolver::new(ns_pool);

    let mut rb = abstract_ns::RouterBuilder::new();
    rb.add_default(ns);
    let resolver = rb.into_resolver();

    let http_pools = HttpPools::new();
    let session_pools = chat::SessionPools::new();
    let runtime = Arc::new(Runtime {
        config: cfg.clone(),
        handle: handle.clone(),
        http_pools: http_pools.clone(),
        session_pools: session_pools.clone(),
        meter: meter,
    });
    let root = cfg.get();

    let mut listener_shutters = HashMap::new();

    // TODO(tailhook) do something when config updates
    for sock in &root.listen {
        match sock {
            &ListenSocket::Tcp(addr) => {
                if verbose {
                    println!("Listening at {}", addr);
                }
                let (tx, rx) = oneshot();
                // TODO(tailhook) wait and retry on error
                match spawn_listener(addr, handle, &runtime, rx) {
                    Ok(()) => {
                        listener_shutters.insert(addr, tx);
                    }
                    Err(e) => {
                        error!("Error listening {}: {}. Will retry on next \
                                configuration reload", addr, e);
                    }
                }
            }
        }
    }


    handlers::files::update_pools(&root.disk_pools);
    http_pools.update(&root.http_destinations, &resolver, handle);
    session_pools.update(&root.session_pools, handle, &runtime);
    State {
        ns: resolver,
        http_pools: http_pools,
        session_pools: session_pools,
        listener_shutters: listener_shutters,
        runtime: runtime,
    }
}

pub fn update_loop(state: &mut State, cfg: &ConfigCell, handle: &Handle) {
    // TODO(tailhook) update listening sockets
    handlers::files::update_pools(&cfg.get().disk_pools);
    state.http_pools.update(&cfg.get().http_destinations, &state.ns, handle);
    state.session_pools.update(&cfg.get().session_pools,
        handle, &state.runtime);
}
