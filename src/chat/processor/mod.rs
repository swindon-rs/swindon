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

use rustc_serialize::json::Json;
use tokio_core::channel::channel as tokio_channel;

use config;
use intern::Atom;
use chat::Cid;

mod main;
mod pool;
mod public;
mod session;
mod heap;
mod try_iter;  // temporary
mod connection;

pub use self::public::{Processor, ProcessorPool};


pub struct Event {
    pool: Atom,
    timestamp: Instant,
    action: Action,
}

#[derive(Debug)]
pub enum Action {

    // ------ Session pool management ------
    //   For all actions session pool name is passed in event structure
    EnsureSessionPool(Arc<config::SessionPool>),
    StopSessionPool,

    // ------ Connection management ------
    NewConnection {
        conn_id: Cid,
    },
    Associate {
        conn_id: Cid,
        session_id: Atom,
        metadata: Arc<Json>
    },
    UpdateActivity {
        conn_id: Cid,
        // We receive duration from client, but we expect request handling
        // code to validate and normalize it for us
        timestamp: Instant,
    },
    Disconnect {
        conn_id: Cid,
    },

    // ------ Subscriptions ------
    Subscribe {
        conn_id: Cid,
        topic: Atom,
    },
    Unsubscribe {
        conn_id: Cid,
        topic: Atom,
    },
    Publish {
        topic: Atom,
        data: Arc<Json>,
    },
}
