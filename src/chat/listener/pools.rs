use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use async_slot as slot;
use futures::Stream;
use futures::sync::oneshot::{Sender};
use futures::sync::mpsc::{unbounded as channel};
use tokio_core::reactor::Handle;

use crate::runtime::Runtime;
use crate::intern::SessionPoolName;
use crate::chat::listener::spawn::{listen, WorkerData};
use crate::chat::inactivity_handler;
use crate::chat::processor::{Processor};
use crate::chat::Shutdown;
use crate::chat::replication::RemoteSender;
use crate::config::listen::Listen;
use crate::config::{SessionPool};
use void::Void;


#[derive(Clone)]
pub struct SessionPools {
    pools: Arc<RwLock<HashMap<SessionPoolName, Worker>>>,
    pub processor: Processor,
    pub remote_sender: RemoteSender,
}

struct Worker {
    listener_channel: slot::Sender<Listen>,
    inactivity_shutter: Sender<Shutdown>,
}

impl SessionPools {
    pub fn new(processor: Processor, remote_sender: RemoteSender)
        -> SessionPools
    {
        SessionPools {
            pools: Arc::new(RwLock::new(HashMap::new())),
            processor: processor,
            remote_sender: remote_sender,
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
                drop(wrk.listener_channel);
                wrk.inactivity_shutter.send(Shutdown).ok();
            }
        }

        // Create new pools
        for (name, settings) in cfg {
            if let Some(ref mut pool) = pools.get(name) {
                pool.listener_channel.swap(settings.listen.clone())
                    .map_err(|_| error!("Can't update addresses for {}",
                                        name))
                    .ok();
                continue;
            }
            let (tx, rx) = channel();
            self.processor.create_pool(name, settings, tx);
            let in_shutter = inactivity_handler::run(
                runtime, settings, handle, rx);

            let (listen_tx, listen_rx) = slot::channel();
            listen_tx.swap(settings.listen.clone()).unwrap();
            let wdata = Arc::new(WorkerData {
                name: name.clone(),
                runtime: runtime.clone(),
                settings: settings.clone(),
                processor: self.processor.pool(name),
                remote: self.remote_sender.pool(name),
                handle: handle.clone(),
            });
            listen(
                runtime.resolver.subscribe_stream(
                    listen_rx.map_err(|()| -> Void { unreachable!() }), 80),
                &wdata);

            pools.insert(name.clone(), Worker {
                listener_channel: listen_tx,
                inactivity_shutter: in_shutter,
            });
        }
    }
}
