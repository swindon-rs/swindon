use std::thread::spawn;
use std::sync::Arc;
use std::time::Instant;
use std::sync::mpsc::{channel, Sender};
use std::collections::HashSet;

use tokio_core::channel::Sender as TokioSender;

use intern::Atom;
use config;
use super::{Event, Action, PoolMessage};
use super::main;


pub struct Processor {
    pools: HashSet<Atom>,
    queue: Sender<Event>,
}

#[derive(Clone)]
pub struct ProcessorPool {
    pool_name: Atom,
    queue: Sender<Event>,
}


impl Processor {
    pub fn new() -> Processor {
        let (tx, rx) = channel();
        spawn(move || {
            main::run(rx)
        });
        return Processor {
            queue: tx,
            pools: HashSet::new(),
        }
    }

    pub fn create_pool(&self, name: &Atom,
        config: &Arc<config::SessionPool>, channel: TokioSender<PoolMessage>)
    {
        self.queue.send(Event {
            pool: name.clone(),
            timestamp: Instant::now(),
            action: Action::NewSessionPool {
                config: config.clone(),
                channel: channel,
            },
        }).map_err(|e| panic!("Processor loop send error: {}", e)).ok();
    }

    pub fn pool(&self, name: &Atom)
        -> ProcessorPool
    {
        assert!(self.pools.contains(name));
        ProcessorPool {
            pool_name: name.clone(),
            // TODO(tailhook) Should we reference Processor instead
            queue: self.queue.clone(),
        }
    }

    pub fn has_pool(&self, name: &Atom) -> bool {
        self.pools.contains(name)
    }
}

impl ProcessorPool {
    pub fn send(&self, action: Action) {
        self.queue.send(Event {
            pool: self.pool_name.clone(),
            timestamp: Instant::now(),
            action: action,
        }).map_err(|e| panic!("Error invoking processor: {}", e)).ok();
    }
}