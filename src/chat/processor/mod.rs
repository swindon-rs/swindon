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
use std::sync::Arc;
use std::collections::HashMap;

use rustc_serialize::json::Json;
use tokio_core::channel::channel as tokio_channel;

use config;
use intern::Atom;
use chat::Cid;

mod main;
mod pool;
mod public;

pub use self::public::Processor;


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

    // Connection management
    NewConnection {
        user_id: Atom,
        conn_id: Cid,
        metadata: Arc<Json>
    },
}
