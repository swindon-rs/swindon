mod authorize;
mod backend;
mod cid;
mod close_reason;
mod connection_sender;
mod content_type;
mod dispatcher;
mod error;
mod inactivity_handler;
mod listener;
mod message;
mod processor;
mod replication;
pub mod tangle_auth;

pub use self::cid::Cid;
pub use self::authorize::{start_authorize, good_status};
pub use self::message::{Meta, Args, Kwargs};
pub use self::error::MessageError;
pub use self::close_reason::CloseReason;
pub use self::listener::SessionPools;
pub use self::processor::{Processor, ConnectionMessage, json_err};
pub use self::dispatcher::Dispatcher;
pub use self::connection_sender::ConnectionSender;
pub use self::replication::ReplicationSession;

use crate::metrics::{Counter, Integer, List, Metric};
use crate::prometheus_metrics as prom;

lazy_static! {
    pub static ref CONNECTS: Counter = Counter::new();
    pub static ref CONNECTIONS: Integer = Integer::new();
    pub static ref FRAMES_SENT: Counter = Counter::new();
}

pub struct Shutdown;

pub fn metrics() -> List {
    vec![
        (Metric("websockets.swindon_chat", "connects"), &*CONNECTS),
        (Metric("websockets.swindon_chat", "connections"), &*CONNECTIONS),
        (Metric("websockets.swindon_chat", "frames_received"),
            &*dispatcher::FRAMES_RECEIVED),
        (Metric("websockets.swindon_chat", "frames_sent"), &*FRAMES_SENT),
        (Metric("websockets.swindon_chat", "session_pools"),
            &*processor::SESSION_POOLS),
        (Metric("websockets.swindon_chat", "active_sessions"),
            &*processor::ACTIVE_SESSIONS),
        (Metric("websockets.swindon_chat", "inactive_sessions"),
            &*processor::INACTIVE_SESSIONS),
        (Metric("websockets.swindon_chat.pubsub", "input_messages"),
            &*processor::PUBSUB_INPUT),
        (Metric("websockets.swindon_chat.pubsub", "output_messages"),
            &*processor::PUBSUB_OUTPUT),
        (Metric("websockets.swindon_chat.pubsub", "topics"),
            &*processor::TOPICS),
        (Metric("websockets.swindon_chat.lattice", "namespaces"),
            &*processor::LATTICES),
        (Metric("websockets.swindon_chat.lattice.shared", "keys"),
            &*processor::SHARED_KEYS),
        (Metric("websockets.swindon_chat.lattice.shared", "counters"),
            &*processor::SHARED_COUNTERS),
        (Metric("websockets.swindon_chat.lattice.shared", "sets"),
            &*processor::SHARED_SETS),
        (Metric("websockets.swindon_chat.lattice.shared", "registers"),
            &*processor::SHARED_REGISTERS),
        (Metric("websockets.swindon_chat.lattice.private", "keys"),
            &*processor::PRIVATE_KEYS),
        (Metric("websockets.swindon_chat.lattice.private", "counters"),
            &*processor::PRIVATE_COUNTERS),
        (Metric("websockets.swindon_chat.lattice.private", "sets"),
            &*processor::PRIVATE_SETS),
        (Metric("websockets.swindon_chat.lattice.private", "registers"),
            &*processor::PRIVATE_REGISTERS),
        (Metric("websockets.swindon_chat.lattice", "set_items"),
            &*processor::SET_ITEMS),
        (Metric("replication", "connections"),
            &*replication::CONNECTIONS),
        (Metric("replication", "frames_sent"),
            &*replication::FRAMES_SENT),
        (Metric("replication", "frames_received"),
            &*replication::FRAMES_RECEIVED),
    ]
}

pub fn prometheus_metrics() -> prom::List<'static> {
    vec![
        ("websockets_swindon_chat_connects", &[], &*CONNECTS),
        ("websockets_swindon_chat_connections", &[], &*CONNECTIONS),
        ("websockets_swindon_chat_frames_received", &[],
            &*dispatcher::FRAMES_RECEIVED),
        ("websockets_swindon_chat_frames_sent", &[], &*FRAMES_SENT),
        ("websockets_swindon_chat_session_pools", &[],
            &*processor::SESSION_POOLS),
        ("websockets_swindon_chat_sessions", &[("type", "active")],
            &*processor::ACTIVE_SESSIONS),
        ("websockets_swindon_chat_sessions", &[("type", "incative")],
            &*processor::INACTIVE_SESSIONS),
        ("websockets_swindon_chat_pubsub_input_messages", &[],
            &*processor::PUBSUB_INPUT),
        ("websockets_swindon_chat_pubsub_output_messages", &[],
            &*processor::PUBSUB_OUTPUT),
        ("websockets_swindon_chat_pubsub_topics", &[],
            &*processor::TOPICS),
        ("websockets_swindon_chat_lattice_namespaces", &[],
            &*processor::LATTICES),
        ("websockets_swindon_chat_lattice",
            &[("scope", "shared"), ("type", "keys")],
            &*processor::SHARED_KEYS),
        ("websockets_swindon_chat_lattice",
            &[("scope", "shared"), ("type", "counters")],
            &*processor::SHARED_COUNTERS),
        ("websockets_swindon_chat_lattice",
            &[("scope", "shared"), ("type", "sets")],
            &*processor::SHARED_SETS),
        ("websockets_swindon_chat_lattice",
            &[("scope", "shared"), ("type", "registers")],
            &*processor::SHARED_REGISTERS),
        ("websockets_swindon_chat_lattice",
            &[("scope", "private"), ("type", "keys")],
            &*processor::PRIVATE_KEYS),
        ("websockets_swindon_chat_lattice",
            &[("scope", "private"), ("type", "counters")],
            &*processor::PRIVATE_COUNTERS),
        ("websockets_swindon_chat_lattice",
            &[("scope", "private"), ("type", "sets")],
            &*processor::PRIVATE_SETS),
        ("websockets_swindon_chat_lattice",
            &[("scope", "private"), ("type", "registers")],
            &*processor::PRIVATE_REGISTERS),
        ("websockets_swindon_chat_lattice_set_items", &[],
            &*processor::SET_ITEMS),
        ("replication_connections", &[], &*replication::CONNECTIONS),
        ("replication_frames_sent", &[], &*replication::FRAMES_SENT),
        ("replication_frames_received", &[], &*replication::FRAMES_RECEIVED),
    ]
}
