use std::mem;
use std::sync::{Arc, RwLock};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use tokio_core::reactor::Handle;
use tokio_core::reactor::Interval;
use futures::{Future, Stream};
use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::sync::oneshot::{channel as oneshot, Sender};
use tk_http::websocket::Packet;
use rustc_serialize::json;

use request_id;
use intern::SessionPoolName;
use runtime::{RuntimeId, Runtime};
use config::{ListenSocket, Replication};
use chat::processor::Processor;

use super::{ReplAction, RemoteAction, IncomingChannel, OutgoingChannel};
use super::spawn::{listen, connect};


pub struct ReplicationSession {
    pub watcher: Arc<Watcher>,
    pub remote_sender: RemoteSender,
    runtime_id: RuntimeId,
    shutters: HashMap<SocketAddr, Sender<()>>,
    reconnect_shutter: Option<Sender<()>>,
}

pub struct Watcher {
    // TODO: revise Arc<RwLock<>> on peers -- operations performed often;
    peers: Arc<RwLock<HashMap<SocketAddr, State>>>,
    pub tx: IncomingChannel,
    processor: Processor,
}

#[derive(Debug)]
enum State {
    Unknown,
    Connecting {
        timeout: Instant
    },
    Connected {
        tx: OutgoingChannel,
    },
    Disconnected,
}

#[derive(Clone)]
pub struct RemoteSender {
    queue: UnboundedSender<ReplAction>,
}

pub struct RemotePool {
    pool: SessionPoolName,
    queue: UnboundedSender<ReplAction>,
}

impl ReplicationSession {
    pub fn new(processor: Processor, handle: &Handle)
        -> ReplicationSession
    {
        let (inc_tx, inc_rx) = unbounded();
        let (out_tx, out_rx) = unbounded();
        let watcher = Arc::new(Watcher {
            processor: processor,
            peers: Arc::new(RwLock::new(HashMap::new())),
            tx: inc_tx,
        });
        let w1 = watcher.clone();
        let w2 = watcher.clone();
        let runtime_id = request_id::new();

        handle.spawn(inc_rx.for_each(move |e| {
            w1.handle_incoming(e);
            Ok(())
        }));
        handle.spawn(out_rx.for_each(move |e| {
            w2.handle_outgoing(e);
            Ok(())
        }));

        ReplicationSession {
            runtime_id: runtime_id,
            watcher: watcher,
            remote_sender: RemoteSender { queue: out_tx },
            shutters: HashMap::new(),
            reconnect_shutter: None,
        }
    }

    pub fn update(&mut self, cfg: &Arc<Replication>, _runtime: &Arc<Runtime>,
        handle: &Handle)
    {
        let mut to_delete = Vec::new();
        for (&addr, _) in &self.shutters {
            let laddr = ListenSocket::Tcp(addr);
            if cfg.listen.iter().find(|&x| x == &laddr).is_none() {
                to_delete.push(addr);
            }
        }
        for addr in to_delete {
            if let Some(shutter) = self.shutters.remove(&addr) {
                shutter.send(());
            }
        }
        for addr in &cfg.listen {
            match *addr {
                ListenSocket::Tcp(addr) => {
                    let (tx, rx) = oneshot();
                    match listen(addr, self.watcher.tx.clone(),
                        &self.runtime_id, &cfg, handle, rx)
                    {
                        Ok(()) => {
                            self.shutters.insert(addr, tx);
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

        // TODO:
        // drop/delete removed peers
        // add new to watcher queue

        // stop reconnecting
        if let Some(tx) = self.reconnect_shutter.take() {
            tx.send(());
        }

        let mut peers = self.watcher.peers.write().expect("writable");
        for addr in &cfg.peers {
            match *addr {
                ListenSocket::Tcp(addr) => {
                    peers.insert(addr, State::Unknown);
                }
            }
        }

        let runtime_id = self.runtime_id.clone();
        let w = self.watcher.clone();
        let h = handle.clone();
        let s = cfg.clone();
        let (tx, shutter) = oneshot();
        self.reconnect_shutter = Some(tx);
        handle.spawn(Interval::new(Duration::new(1, 0), &handle)
            .expect("interval created")
            .map_err(|e| error!("Interval error: {}", e))
            .for_each(move |_| {
                w.reconnect(&runtime_id, &s, &h);
                Ok(())
            })
            .select(shutter.map_err(|_| unreachable!()))
            .map(|(_, _)| info!("Reconnector stopped"))
            .map_err(|(_, _)| info!("Reconnector stopped")));
    }

}

impl Watcher {
    pub fn reconnect(&self, runtime_id: &RuntimeId,
        settings: &Arc<Replication>, handle: &Handle)
    {
        let mut peers = self.peers.write().expect("writable");
        // TODO: Configure timeout value;
        let now = Instant::now();
        let timeout = now + *settings.reconnect_timeout;
        for (addr, state) in peers.iter_mut() {
            match *state {
                State::Unknown => {
                    debug!("Connecting new unkown peer: {}", addr);
                }
                State::Disconnected => {
                    debug!("Retrying disconnected peer: {}", addr);
                }
                State::Connecting { ref timeout } => {
                    if timeout < &now {
                        debug!("Retrying timeouted peer: {}", addr);
                    } else {
                        continue
                    }
                },
                State::Connected {..} => continue,
            }
            debug!("Spawn connect({})...", addr);
            mem::replace(state, State::Connecting {
                timeout: timeout.clone(),
            });
            connect(*addr, self.tx.clone(), runtime_id, timeout, handle);
        }
    }

    fn handle_incoming(&self, action: ReplAction) {
        let mut peers = self.peers.write().expect("acquired for update");
        match action {
            ReplAction::Attach { tx, runtime_id, addr } => {
                debug!("Got connection from: {}, {}", addr, runtime_id);
                let s = State::Connected { tx: tx };
                if let Some(prev) = peers.insert(addr, s) {
                    debug!("Replaced prev connection {}: {:?}", addr, prev);
                };
            }
            ReplAction::RemoteAction { pool, action } => {
                self.processor.send(&pool, action.into());
            }
        }
    }

    fn handle_outgoing(&self, action: ReplAction) {
        if let Ok(data) = json::encode(&action) {
            let mut peers = self.peers.write().expect("acquired for update");
            for (_, state) in peers.iter_mut() {
                let err = match *state {
                    State::Connected { ref tx } => {
                        debug!("Publishing data: {:?}", data);
                        tx.send(Packet::Text(data.clone())).is_err()
                    }
                    _ => continue,
                };
                if err {
                    mem::replace(state, State::Disconnected);
                }
            }
        }
    }
}

impl RemoteSender {
    pub fn pool(&self, name: &SessionPoolName) -> RemotePool {
        RemotePool {
            pool: name.clone(),
            queue: self.queue.clone(),
        }
    }
}

impl RemotePool {

    pub fn send(&self, action: RemoteAction) {
        self.queue.send(ReplAction::RemoteAction {
            pool: self.pool.clone(),
            action: action,
        }).map_err(|e| error!("Error sending event: {}", e)).ok();
    }
}
