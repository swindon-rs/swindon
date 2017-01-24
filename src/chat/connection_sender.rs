use futures::sync::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};

use chat::{ConnectionMessage};

pub type Receiver = UnboundedReceiver<ConnectionMessage>;

#[derive(Clone)]
pub struct ConnectionSender {
    sender: UnboundedSender<ConnectionMessage>,
}

impl ConnectionSender {
    pub fn new() -> (ConnectionSender, Receiver) {
        let (tx, rx) = unbounded();
        (ConnectionSender {
            sender: tx,
        }, rx)
    }
    pub fn send(&self, msg: ConnectionMessage) {
        self.sender.send(msg)
        .map_err(|e| debug!("Error sending connection message: {}. \
            usually these means connection has been closed to soon", e)).ok();
    }
}
