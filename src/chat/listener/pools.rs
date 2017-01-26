use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use tokio_core::reactor::Handle;
use futures::sync::oneshot::{channel as oneshot, Sender};
use futures::sync::mpsc::{unbounded as channel};

use runtime::Runtime;
use intern::SessionPoolName;
use chat::listener::spawn::{listen, WorkerData};
use chat::inactivity_handler;
use chat::processor::{Processor, Action};
use chat::Shutdown;
use config::{SessionPool, ListenSocket};


#[derive(Clone)]
pub struct SessionPools {
    pools: Arc<RwLock<HashMap<SessionPoolName, Worker>>>,
    pub processor: Processor,
}

struct Worker {
    data: Arc<WorkerData>,
    shutters: HashMap<SocketAddr, Sender<Shutdown>>,
    inactivity_shutter: Sender<Shutdown>,
}

impl SessionPools {
    pub fn new() -> SessionPools {
        SessionPools {
            pools: Arc::new(RwLock::new(HashMap::new())),
            processor: Processor::new(),
        }
    }
    pub fn update(&self, cfg: &HashMap<SessionPoolName, Arc<SessionPool>>,
        handle: &Handle, runtime: &Arc<Runtime>)
    {
        let mut pools = self.pools.write().expect("pools not poisoned");

        let mut to_delete = Vec::new();
        for k in pools.keys() {
            if !cfg.contains_key(k) {
                to_delete.push(k.clone());
            }
        }

        for k in to_delete {
            if let Some(wrk) = pools.remove(&k) {
                self.processor.destroy_pool(&k);
                for (_, shutter) in wrk.shutters {
                    shutter.complete(Shutdown);
                }
                wrk.inactivity_shutter.complete(Shutdown);
            }
        }

        // Create new pools
        for (name, settings) in cfg {
            if pools.contains_key(name) {
                continue;
            }

            let (tx, rx) = channel();
            self.processor.create_pool(name, settings, tx);
            let in_shutter = inactivity_handler::run(
                runtime, settings, handle, rx);

            pools.insert(name.clone(), Worker {
                data: Arc::new(WorkerData {
                    name: name.clone(),
                    runtime: runtime.clone(),
                    settings: settings.clone(),
                    processor: self.processor.pool(name),
                    handle: handle.clone(),
                }),
                shutters: HashMap::new(),
                inactivity_shutter: in_shutter,
            });
        }

        // listen sockets
        for (name, settings) in cfg {
            let worker = pools.get_mut(name).unwrap();

            let mut to_delete = Vec::new();
            for (&addr, _) in &worker.shutters {
                let laddr = ListenSocket::Tcp(addr);
                if settings.listen.iter().find(|&x| x == &laddr).is_none() {
                    to_delete.push(addr);
                }
            }
            for addr in to_delete {
                if let Some(shutter) = worker.shutters.remove(&addr) {
                    shutter.complete(Shutdown);
                }
            }

            for addr in &settings.listen {
                match *addr {
                    ListenSocket::Tcp(addr) => {
                        let (tx, rx) = oneshot();
                        // TODO(tailhook) wait and retry on error
                        match listen(addr, &worker.data, rx) {
                            Ok(()) => {
                                worker.shutters.insert(addr, tx);
                            }
                            Err(e) => {
                                error!("Error listening {}: {}. \
                                    Will retry on next config reload",
                                    addr, e);
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn send(&self, pool: &SessionPoolName, action: Action) {
        self.processor.send(pool, action)
    }
}
