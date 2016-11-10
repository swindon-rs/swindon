use std::thread::spawn;
use std::sync::Arc;
use std::time::Instant;
use std::sync::mpsc::{channel, Sender};
use std::collections::HashMap;

use intern::Atom;
use config;
use super::{Event, Action};
use super::main;


#[derive(Clone)]
pub struct Processor {
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

    pub fn update_pools(&self, pools: &HashMap<Atom, Arc<config::SessionPool>>)
    {
        for (name, props) in pools {
            self.queue.send(Event {
                pool: name.clone(),
                timestamp: Instant::now(),
                action: Action::EnsureSessionPool(props.clone()),
            }).map_err(|e| panic!("Processor loop send error: {}", e)).ok();
        }
    }
}
