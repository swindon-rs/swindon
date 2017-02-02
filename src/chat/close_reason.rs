use minihttp::Status;


#[derive(Debug, Clone, PartialEq)]
pub enum CloseReason {
    /// Stopping websocket because respective session pool is stopped
    PoolStopped,
    /// Closing because respective http returned specified response code
    AuthHttp(Status),
    /// Closed by peer, we just propagate the message here
    PeerClose(u16, String),
}

impl CloseReason {
    pub fn code(&self) -> u16 {
        use self::CloseReason::*;
        match *self {
            PoolStopped => 4001,
            AuthHttp(code) if code.code() >= 400 && code.code() <= 599
            => 4000 + code.code(),
            AuthHttp(_) => 4500,
            PeerClose(x, _) => x,
        }
    }
    pub fn reason(&self) -> &str {
        use self::CloseReason::*;
        match *self {
            PoolStopped => "session_pool_stopped",
            AuthHttp(_) => "backend_error",
            PeerClose(_, ref y) => y,
        }
    }
}
