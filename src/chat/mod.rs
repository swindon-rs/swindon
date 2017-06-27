mod cid;
mod authorize;
mod backend;
mod message;
mod tangle_auth;
mod processor;
mod error;
mod close_reason;
mod listener;
mod dispatcher;
mod connection_sender;
mod inactivity_handler;
mod replication;

pub use self::cid::Cid;
pub use self::tangle_auth::TangleAuth;
pub use self::authorize::start_authorize;
pub use self::message::{Meta, Args, Kwargs};
pub use self::error::MessageError;
pub use self::close_reason::CloseReason;
pub use self::listener::SessionPools;
pub use self::processor::{Processor, ConnectionMessage};
pub use self::dispatcher::Dispatcher;
pub use self::connection_sender::ConnectionSender;
pub use self::replication::ReplicationSession;

use metrics::{Counter, Integer, List, Metric};

lazy_static! {
    pub static ref CONNECTS: Counter = Counter::new();
    pub static ref CONNECTIONS: Integer = Integer::new();
}

pub struct Shutdown;

pub fn metrics() -> List {
    vec![
        (Metric("websockets.swindon_chat", "connects"), &*CONNECTS),
        (Metric("websockets.swindon_chat", "connections"), &*CONNECTIONS),
        (Metric("websockets.swindon_chat", "frames_received"),
            &*dispatcher::FRAMES_RECEIVED),
    ]
}
