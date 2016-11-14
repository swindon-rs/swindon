use std::thread::spawn;
use std::sync::Arc;
use std::time::Instant;
use std::sync::mpsc::{channel, Sender};
use std::collections::HashSet;

use tokio_core::channel::Sender as TokioSender;

use intern::SessionPoolName;
use config;
use super::{Event, Action, PoolMessage};
use super::main;


pub struct Processor {
    pools: HashSet<SessionPoolName>,
    queue: Sender<Event>,
}

#[derive(Clone)]
pub struct ProcessorPool {
    pool_name: SessionPoolName,
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

    pub fn create_pool(&mut self, name: &SessionPoolName,
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
        self.pools.insert(name.clone());
    }

    pub fn pool(&self, name: &SessionPoolName)
        -> ProcessorPool
    {
        if !self.pools.contains(name) {
            panic!("No pool {} defined", name);
        }
        ProcessorPool {
            pool_name: name.clone(),
            // TODO(tailhook) Should we reference Processor instead
            queue: self.queue.clone(),
        }
    }

    pub fn has_pool(&self, name: &SessionPoolName) -> bool {
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
