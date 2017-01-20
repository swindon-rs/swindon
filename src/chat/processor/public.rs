use std::thread::spawn;
use std::sync::Arc;
use std::time::Instant;
use std::sync::mpsc::{channel, Sender};
use futures::sync::mpsc::{UnboundedSender as ChannelSender};


use intern::SessionPoolName;
use config;
use super::{Event, Action, PoolMessage};
use super::main;


#[derive(Clone)]
pub struct Processor {
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
        }
    }

    pub fn create_pool(&self, name: &SessionPoolName,
        config: &Arc<config::SessionPool>, channel: ChannelSender<PoolMessage>)
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

    pub fn destroy_pool(&self, name: &SessionPoolName) {
        self.queue.send(Event {
            pool: name.clone(),
            timestamp: Instant::now(),
            action: Action::StopSessionPool,
        }).map_err(|e| panic!("Processor loop send error: {}", e)).ok();
    }

    pub fn pool(&self, name: &SessionPoolName)
        -> ProcessorPool
    {
        ProcessorPool {
            pool_name: name.clone(),
            // TODO(tailhook) Should we reference Processor instead
            queue: self.queue.clone(),
        }
    }

    /// Send directly without getting pool
    pub fn send(&self, pool: &SessionPoolName, action: Action) {
        debug!("Sending pool action {:?} {:?}", pool, action);
        self.queue.send(Event {
            pool: pool.clone(),
            timestamp: Instant::now(),
            action: action,
        }).map_err(|e| panic!("Error invoking processor: {}", e)).ok();
    }
}

impl ProcessorPool {
    pub fn send(&self, action: Action) {
        debug!("Sending pool action {:?} {:?}", self.pool_name, action);
        self.queue.send(Event {
            pool: self.pool_name.clone(),
            timestamp: Instant::now(),
            action: action,
        }).map_err(|e| panic!("Error invoking processor: {}", e)).ok();
    }
}
