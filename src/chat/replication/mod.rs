use futures::sync::mpsc::UnboundedSender;
use tk_http::websocket::Packet;

mod action;
mod session;
mod spawn;
mod server;
mod client;
mod serialize;

pub use self::action::{ReplAction, RemoteAction};
pub use self::session::ReplicationSession;
pub use self::session::{RemoteSender, RemotePool};

pub type IncomingChannel = UnboundedSender<ReplAction>;
pub type OutgoingChannel = UnboundedSender<Packet>;
