mod handshake;
mod base64;
mod dispatcher;
mod proto;
mod write;
// dispatchers
mod echo;

pub use self::handshake::{Init, prepare, negotiate};
pub use self::dispatcher::{Dispatcher, ImmediateReplier};
pub use self::echo::Echo;
pub use self::proto::{Frame, Error};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    Echo,
}
