use futures::sync::mpsc::{UnboundedReceiver as Receiver};
use minihttp::Status;

use super::websocket::Init;

mod api;
mod backend;
mod message;
mod websocket;
mod router;
mod processor;
mod error;

pub use self::backend::ChatBackend;
pub use self::processor::{Processor, ProcessorPool, Action};
pub use self::websocket::{negotiate, fail};
pub use self::router::MessageRouter;
pub use self::message::{Meta, Args, Kwargs};
pub use self::api::{ChatAPI, SessionAPI, MaintenanceAPI, parse_userinfo};
pub use self::error::MessageError;

use self::processor::ConnectionMessage;

/// Internal connection id
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Cid(u64);

pub enum ChatInit {
    Prepare(Init, ChatAPI),
    Ready(Init, SessionAPI, Receiver<ConnectionMessage>),
    AuthError(Init, Status),
}


impl Cid {
    #[cfg(target_pointer_width = "64")]
    pub fn new() -> Cid {
        // Until atomic u64 really works
        use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
        static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        Cid(COUNTER.fetch_add(1, Ordering::Relaxed) as u64)
    }
}

// TODO: make this two functions properly serialize and deserialize Cid;
pub fn serialize_cid(cid: &Cid) -> String {
    format!("{}", cid.0)
}

pub fn parse_cid(raw: String) -> Cid {
    Cid(raw.parse().unwrap())
}
