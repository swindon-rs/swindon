use std::str::FromStr;
use futures::sync::mpsc::{UnboundedReceiver as Receiver};
use minihttp::Status;

mod cid;
mod authorize;
//mod api;
mod backend;
mod message;
mod processor;
mod error;
mod close_reason;
mod listener;

pub use self::cid::Cid;
pub use self::authorize::start_authorize;
//pub use self::backend::ChatBackend;
pub use self::processor::{Processor, ProcessorPool, Action};
pub use self::message::{Meta, Args, Kwargs};
//pub use self::api::{ChatAPI, SessionAPI, MaintenanceAPI, parse_userinfo};
pub use self::error::MessageError;
pub use self::close_reason::CloseReason;
pub use self::listener::spawn_listener;

//use self::processor::ConnectionMessage;
