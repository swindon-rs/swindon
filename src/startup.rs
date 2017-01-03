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

use config::{ListenSocket, Handler, ConfigCell};
use incoming::Router;
//use chat::{ChatBackend, Processor, MaintenanceAPI};
use handlers;
use runtime::Runtime;
use http_pools::{HttpPools};


pub struct State {
    //chat: Arc<RwLock<Processor>>,
    http_pools: HttpPools,
    ns: abstract_ns::Router,
    listener_shutters: HashMap<SocketAddr, Sender<()>>,
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

    //let chat_pro = Arc::new(RwLock::new(Processor::new()));
    let http_pools = HttpPools::new();
    let runtime = Arc::new(Runtime {
        config: cfg.clone(),
        handle: handle.clone(),
        http_pools: http_pools.clone(),
        //chat_processor: chat_pro.clone(),
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

    /*
    for (name, cfg) in &root.session_pools {
        let (tx, rx) = channel();
        chat_pro.write().unwrap().create_pool(name, cfg, tx);
        let maintenance = MaintenanceAPI::new(
            root.clone(), cfg.clone(), http_pools.clone(),
            handle.clone());
        handle.spawn(rx.for_each(move |msg| {
            maintenance.handle(msg);
            Ok(())
        }))
    }
    for (name, h) in root.handlers.iter() {
        if let &Handler::SwindonChat(ref chat) = h {
            let sess = root.session_pools
                .get(&chat.session_pool).unwrap();
            match sess.listen {
                ListenSocket::Tcp(addr) => {
                    if verbose {
                        println!("Listening {} at {}", name, addr);
                    }
                    let chat_handler = ChatBackend {
                        config: cfg.clone(),
                        chat_pool: chat_pro.read().unwrap().pool(
                            &chat.session_pool),
                    };
                    minihttp::serve(handle, addr,
                        move || Ok(chat_handler.clone()));
                }
            }
        }
    }
    */
    handlers::files::update_pools(&cfg.get().disk_pools);
    http_pools.update(&cfg.get().http_destinations, &resolver, handle);
    State {
        //chat: chat_pro,
        ns: resolver,
        http_pools: http_pools,
        listener_shutters: listener_shutters,
    }
}
pub fn update_loop(state: &mut State, cfg: &ConfigCell, handle: &Handle) {
    // TODO(tailhook) update listening sockets
    handlers::files::update_pools(&cfg.get().disk_pools);
/*
    state.http_pools.update(&cfg.get().http_destinations, &state.ns, handle);
    let mut chat_pro = state.chat.write().unwrap();
    let config = cfg.get();
    for (name, cfg) in &config.session_pools {
        if !chat_pro.has_pool(name) {
            let (tx, rx) = channel();
            chat_pro.create_pool(name, cfg, tx);
            let maintenance = MaintenanceAPI::new(
                config.clone(), cfg.clone(), state.http_pools.clone(),
                handle.clone());
            handle.spawn(rx.for_each(move |msg| {
                maintenance.handle(msg);
                Ok(())
            }));
        }
    }
    // TODO(tailhook) update chat handlers
*/
}
