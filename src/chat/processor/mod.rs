//! This is a thread that processes websocket messages
//!
//! Important things about this thread:
//!
//! 1. It's fully driven by input message
//! 2. No timers here
//! 3. No time management here, only accept time value from events (*)
//!
//! This allows much better unit tests
use std::time::Instant;
use std::thread::spawn;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender};
use std::collections::HashMap;

use tokio_core::channel::channel as tokio_channel;

use config;
use intern::Atom;

mod main;
mod pool;

#[derive(Clone)]
pub struct Processor {
    queue: Sender<Event>,
}

pub struct Event {
    pool: Atom,
    timestamp: Instant,
    action: Action,
}

#[derive(Debug)]
pub enum Action {

    // Session pool management
    //   For all actions session pool name is passed in event structure
    EnsureSessionPool(Arc<config::SessionPool>),
    StopSessionPool,

    // Connection actions
}

pub fn start() -> Processor {
    let (tx, rx) = channel();
    spawn(move || {
        main::run(rx)
    });
    return Processor {
        queue: tx,
    }
}

pub fn update_pools(pro: &Processor,
    pools: &HashMap<Atom, Arc<config::SessionPool>>)
{
    let tx = &pro.queue;
    for (name, props) in pools {
        tx.send(Event {
            pool: name.clone(),
            timestamp: Instant::now(),
            action: Action::EnsureSessionPool(props.clone()),
        }).map_err(|e| panic!("Processor loop send error: {}", e));
    }
}
