use std::sync::{Arc, RwLock};

use abstract_ns;
use ns_std_threaded;
use tokio_core::reactor::Handle;
use futures::Stream;
use futures::sync::mpsc::{unbounded as channel};
use futures_cpupool;

use config::{ListenSocket, Handler, ConfigCell};
use handler::Main;
use chat::{ChatBackend, Processor, MaintenanceAPI};
use minihttp;
use handlers;
use http_pools::{HttpPools};


pub struct State {
    chat: Arc<RwLock<Processor>>,
    http_pools: HttpPools,
    ns: abstract_ns::Router,
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

    let chat_pro = Arc::new(RwLock::new(Processor::new()));
    let http_pools = HttpPools::new();
    let main_handler = Main {
        config: cfg.clone(),
        handle: handle.clone(),
        http_pools: http_pools.clone(),
        chat_processor: chat_pro.clone(),
    };
    // TODO(tailhook) do something when config updates
    for sock in &cfg.get().listen {
        match sock {
            &ListenSocket::Tcp(addr) => {
                if verbose {
                    println!("Listening at {}", addr);
                }
                let main_handler = main_handler.clone();
                minihttp::serve(handle, addr,
                    move || Ok(main_handler.clone()));
            }
        }
    }
    let root = cfg.get();
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
    handlers::files::update_pools(&cfg.get().disk_pools);
    http_pools.update(&cfg.get().http_destinations, &resolver, handle);
    State {
        chat: chat_pro,
        ns: resolver,
        http_pools: http_pools,
    }
}

pub fn update_loop(state: &mut State, cfg: &ConfigCell, handle: &Handle) {
    // TODO(tailhook) update listening sockets
    handlers::files::update_pools(&cfg.get().disk_pools);
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
}
