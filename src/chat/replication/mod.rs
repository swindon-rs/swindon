use futures::sync::mpsc::UnboundedSender;
use tk_http::websocket::Packet;

use metrics::{Counter, Integer};

mod action;
mod session;
mod spawn;
mod server;
mod client;

pub use self::action::{ReplAction, RemoteAction};
pub use self::session::ReplicationSession;
pub use self::session::{RemoteSender, RemotePool};

pub type IncomingChannel = UnboundedSender<ReplAction>;
pub type OutgoingChannel = UnboundedSender<Packet>;

lazy_static! {
    pub static ref CONNECTIONS: Integer = Integer::new();
    pub static ref FRAMES_SENT: Counter = Counter::new();
    pub static ref FRAMES_RECEIVED: Counter = Counter::new();
}
