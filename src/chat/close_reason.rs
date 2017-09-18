#[derive(Debug, Clone, PartialEq)]
pub enum CloseReason {
    /// Stopping websocket because respective session pool is stopped
    PoolStopped,
    /// Closed by peer, we just propagate the message here
    PeerClose(u16, String),
}
