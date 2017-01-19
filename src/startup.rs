use std::io;
use std::sync::{Arc, RwLock};
use std::net::SocketAddr;
use std::collections::HashMap;

use abstract_ns;
use ns_std_threaded;
use tokio_core::reactor::{Handle, Timeout};
use tokio_core::net::TcpListener;
use futures::Stream;
use futures::future::{Either, Future, ok};
use futures::sync::mpsc::{unbounded as channel};
use futures::sync::oneshot::{channel as oneshot, Sender, Receiver};
use futures_cpupool;
use minihttp;
use minihttp::server::Proto;

use intern::SessionPoolName;
use config::{ListenSocket, Handler, ConfigCell};
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
    let runtime = runtime.clone();
    let listener = TcpListener::bind(&addr, &handle)?;
    // TODO(tailhook) how to update?
    let mut hcfg = minihttp::server::Config::new()
        .inflight_request_limit(root.pipeline_depth)
        // TODO(tailhook) make it configurable?
        .inflight_request_prealoc(0)
        .done();
    let h1 = handle.clone();

    handle.spawn(
        listener.incoming()
        .then(move |item| match item {
            Ok((socket, saddr)) => {
                ok(Proto::new(socket, &hcfg,
                    Router::new(saddr, runtime.clone(), h1.clone())))
            }
            Err(e) => {
                info!("Error accepting: {}", e);
                unimplemented!();
                /*
                let dur = runtime.config.get().listen_error_timeout;
                Either::B(Timeout::new(*dur, &runtime.handle).unwrap()
                    .from_err()
                    .and_then(|()| Ok(())))
                */
            }
        })
        .buffer_unordered(root.max_connections)
        .for_each(|()| Ok(()))
        .select(shutter.map_err(|_| unreachable!()))
        .map(move |(_, _)| info!("Listener {} exited", addr))
        .map_err(move |(_, _)| info!("Listener {} exited", addr))
    );
    Ok(())
}

pub fn populate_loop(handle: &Handle, cfg: &ConfigCell, verbose: bool)
    -> State
{
    // TODO(tailhook) configure it
    let ns = ns_std_threaded::ThreadedResolver::new(
        futures_cpupool::CpuPool::new(5));
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
