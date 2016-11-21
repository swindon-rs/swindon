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
use rustc_serialize::{Encodable, Encoder};
use tokio_core::channel::Sender;

use config;
use intern::{Topic, SessionId, SessionPoolName};
use chat::Cid;
use chat::message::Meta;

mod main;
mod pool;
mod public;
mod session;
mod heap;
mod try_iter;  // temporary
mod connection;

pub use self::public::{Processor, ProcessorPool};


pub struct Event {
    pool: SessionPoolName,
    timestamp: Instant,
    action: Action,
}

pub enum ConnectionMessage {
    Publish(Arc<Json>),
    /// Auth response message: `["hello", {}, json_data]`
    Hello(Arc<Json>),
    /// Websocket message result;
    Result(Meta, Json),
    // // Result of Auth call;
    // Hello(Arc<Json>),
    // // Lattice update message;
    // Lattice(Arc<Json>),
    // // Some Error
    Error(Json),
}

pub enum PoolMessage {
    InactiveSession {
        session_id: SessionId,
        // This is mostly for debugging for now
        connections_active: usize,
        metadata: Arc<Json>,
    },
}

pub enum Action {

    // ------ Session pool management ------
    //   For all actions session pool name is passed in event structure
    NewSessionPool {
        config: Arc<config::SessionPool>,
        channel: Sender<PoolMessage>,
    },
    StopSessionPool,

    // ------ Connection management ------
    NewConnection {
        conn_id: Cid,
        channel: Sender<ConnectionMessage>,
    },
    Associate {
        conn_id: Cid,
        session_id: SessionId,
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
        topic: Topic,
    },
    Unsubscribe {
        conn_id: Cid,
        topic: Topic,
    },
    Publish {
        topic: Topic,
        data: Arc<Json>,
    },
}

impl Encodable for ConnectionMessage {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error>
    {
        use self::ConnectionMessage::*;
        s.emit_seq(3, |s| {
            match *self {
                Publish(ref json) => {
                    s.emit_seq_elt(0, |s| s.emit_str("message"))?;
                    s.emit_seq_elt(1, |s| s.emit_map(0, |s| Ok(())))?;
                    s.emit_seq_elt(2, |s| json.encode(s))
                }
                Hello(ref json) => {
                    s.emit_seq_elt(0, |s| s.emit_str("hello"))?;
                    s.emit_seq_elt(1, |s| s.emit_map(0, |s| Ok(())))?;
                    s.emit_seq_elt(2, |s| json.encode(s))
                }
                Result(ref meta, ref json) => {
                    s.emit_seq_elt(0, |s| s.emit_str("result"))?;
                    s.emit_seq_elt(1, |s| meta.encode(s))?;
                    s.emit_seq_elt(2, |s| json.encode(s))
                }
                Error(ref json) => {
                    s.emit_seq_elt(0, |s| s.emit_str("Error"))?;
                    s.emit_seq_elt(1, |s| s.emit_map(0, |s| Ok(())))?;  // meta.encode(s))?;
                    s.emit_seq_elt(2, |s| json.encode(s))
                }
            }
        })
    }
}
