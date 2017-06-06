use std::sync::Arc;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use tokio_core::reactor::Handle;
use tokio_core::reactor::Interval;
use futures::{Future, Stream};
use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::sync::oneshot::{channel as oneshot, Sender};
use tk_http::websocket::Packet;
use serde_json::to_string as json_encode;
use abstract_ns::{Router};

use request_id;
use intern::SessionPoolName;
use runtime::{RuntimeId};
use config::{ListenSocket, Replication};
use chat::processor::Processor;

use super::{ReplAction, RemoteAction, IncomingChannel, OutgoingChannel};
use super::action::Message;
use super::spawn::{listen, connect};


pub struct ReplicationSession {
    pub remote_sender: RemoteSender,
    tx: IncomingChannel,
    runtime_id: RuntimeId,
    shutters: HashMap<SocketAddr, Sender<()>>,
    reconnect_shutter: Option<Sender<()>>,
}

pub struct Watcher {
    peers: HashMap<String, State>,
    links: HashMap<RuntimeId, OutgoingChannel>,
    pub tx: IncomingChannel,
    processor: Processor,
    runtime_id: RuntimeId,
    resolver: Router,
    handle: Handle,
}

#[derive(Debug)]
enum State {
    /// Connect started for outbound connection.
    /// RuntimeId is still unknown.
    Connecting(Instant),
    /// Either outbound or inbound live connection.
    /// RuntimeId is known.
    /// Also for outbound connection peer name is known.
    Connected(RuntimeId),
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
    pub fn new(processor: Processor, resolver: &Router, handle: &Handle)
        -> ReplicationSession
    {
        let runtime_id = request_id::new();
        let (tx, rx) = unbounded();
        let mut watcher = Watcher {
            processor: processor,
            peers: HashMap::new(),
            links: HashMap::new(),
            tx: tx.clone(),
            runtime_id: runtime_id.clone(),
            handle: handle.clone(),
            resolver: resolver.clone(),
        };
        handle.spawn(rx.for_each(move |action| {
            watcher.process(action);
            Ok(())
        }));

        ReplicationSession {
            runtime_id: runtime_id,
            tx: tx.clone(),
            remote_sender: RemoteSender { queue: tx },
            shutters: HashMap::new(),
            reconnect_shutter: None,
        }
    }

    pub fn update(&mut self, cfg: &Arc<Replication>, handle: &Handle)
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
                shutter.send(()).ok();
            }
        }
        for addr in &cfg.listen {
            match *addr {
                ListenSocket::Tcp(addr) => {
                    let (tx, rx) = oneshot();
                    match listen(addr, self.tx.clone(),
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

        // stop reconnecting
        if let Some(tx) = self.reconnect_shutter.take() {
            tx.send(()).ok();
        }
        let (tx, shutter) = oneshot();
        self.reconnect_shutter = Some(tx);
        let s = cfg.clone();
        let tx = self.tx.clone();
        handle.spawn(Interval::new(Duration::new(1, 0), &handle)
            .expect("interval created")
            .map(move |_| ReplAction::Reconnect(s.clone()))
            .map_err(|e| error!("Interval error: {}", e))
            .for_each(move |action| {
                tx.send(action)
                .map_err(|_| error!("Error sending action"))
                .ok();
                Ok(())
            })
            .select(shutter.map_err(|_| unreachable!()))
            .map(|_| info!("Reconnector stopped"))
            .map_err(|_| info!("Reconnector stopped"))
        );
    }

}

impl Watcher {

    fn process(&mut self, action: ReplAction) {
        match action {
            ReplAction::Attach { tx, runtime_id, peer } => {
                debug!("Got connection from: {:?}:{}", peer, runtime_id);
                self.attach(tx, runtime_id, peer);
            }
            ReplAction::Incoming(msg) => {
                debug!("Received incoming message: {:?}", msg);
                self.processor.send(&msg.0, msg.1.into());
            }
            ReplAction::Outgoing(msg) => {
                debug!("Sending outgoing message: {:?}", msg);
                self.remote_send(msg);
            }
            ReplAction::Reconnect(ref cfg) => {
                self.reconnect(cfg);
            }
        }
    }

    fn attach(&mut self, tx: OutgoingChannel,
        runtime_id: RuntimeId, peer: Option<String>)
    {
        if let Some(peer) = peer {
            self.peers.insert(peer, State::Connected(runtime_id));
        }
        self.links.insert(runtime_id, tx);
    }

    fn remote_send(&mut self, msg: Message) {
        if let Ok(data) = json_encode(&msg) {
            // TODO: use HashMap::retain() when in stable
            let to_delete = self.links.iter().filter_map(|(remote, tx)| {
                tx.send(Packet::Text(data.clone())).err()
                .map(|_| remote.clone())    // XXX
            }).collect::<Vec<_>>();         // XXX
            for remote in to_delete {
                self.links.remove(&remote);
            }
        } else {
            debug!("error encoding message: {:?}", msg);
        }
    }

    fn reconnect(&mut self, settings: &Arc<Replication>)
    {
        use self::State::*;

        let now = Instant::now();
        let timeout = now + *settings.reconnect_timeout;

        // TODO: use HashMap::retain() when in stable
        let to_delete = self.peers.keys()
            .filter(|p| !settings.peers.contains(p))
            .map(|p| p.clone()).collect::<Vec<_>>();  // XXX
        for peer in to_delete {
            match self.peers.remove(&peer) {
                Some(Connected(runtime_id)) => {
                    self.links.remove(&runtime_id);
                }
                _ => continue,
            }
        };

        for peer in &settings.peers {
            match self.peers.get(peer) {
                Some(&Connected(ref runtime_id)) => {
                    if let Some(_) = self.links.get(runtime_id) {
                        continue
                    }
                }
                Some(&Connecting(ref timeout)) => {
                    if timeout >= &now {
                        continue
                    }
                }
                _ => {}
            };
            self.peers.insert(peer.clone(), Connecting(timeout));
            connect(peer, self.tx.clone(), &self.runtime_id,
                timeout, &self.handle, &self.resolver);
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
        let msg = Message(self.pool.clone(), action);
        self.queue.send(ReplAction::Outgoing(msg))
            .map_err(|e| error!("Error sending event: {}", e)).ok();
    }
}
