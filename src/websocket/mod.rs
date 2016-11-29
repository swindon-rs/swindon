mod handshake;
mod base64;
mod dispatcher;
mod proto;
// not sure this has to be public
pub mod write;
// dispatchers
mod echo;

pub use self::handshake::{Init, prepare, negotiate};
pub use self::dispatcher::{Dispatcher, ImmediateReplier};
pub use self::dispatcher::{RemoteReplier, OutFrame};
pub use self::echo::Echo;
pub use self::proto::{Frame, Error, WebsockProto};
pub use self::base64::Base64;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    Echo,
}

#[derive(Debug, Clone, PartialEq, Eq, RustcEncodable)]
pub enum CloseReason {
    /// Stopping websocket because respective session pool is stopped
    PoolStopped,
    /// Closing because respective http returned specified response code
    AuthHttp(u16),
}

impl CloseReason {
    pub fn code(&self) -> u16 {
        use self::CloseReason::*;
        match *self {
            PoolStopped => 4001,
            AuthHttp(code) if code >= 400 && code <= 599 => 4000 + code,
            AuthHttp(_) => 4500,
        }
    }
    pub fn reason(&self) -> &'static str {
        use self::CloseReason::*;
        match *self {
            PoolStopped => "session_pool_stopped",
            AuthHttp(_) => "backend_error",
        }
    }
}
