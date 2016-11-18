use std::sync::Arc;
use std::collections::HashSet;

use rustc_serialize::json::Json;
use tokio_core::channel::Sender;

use chat::Cid;
use intern::{Topic, SessionId};
use super::ConnectionMessage;


pub struct NewConnection {
    pub cid: Cid,
    pub topics: HashSet<Topic>,
    pub message_buffer: Vec<Arc<Json>>,
    pub channel: Sender<ConnectionMessage>,
}


pub struct Connection {
    pub cid: Cid,
    pub session_id: SessionId,
    pub topics: HashSet<Topic>,
    pub channel: Sender<ConnectionMessage>,
}

impl NewConnection {
    pub fn new(conn_id: Cid, channel: Sender<ConnectionMessage>)
        -> NewConnection
    {
        NewConnection {
            cid: conn_id,
            topics: HashSet::new(),
            message_buffer: Vec::new(),
            channel: channel,
        }
    }
    pub fn associate(self, session_id: SessionId) -> Connection {
        let conn = Connection {
            cid: self.cid,
            session_id: session_id,
            topics: self.topics,
            channel: self.channel,
        };
        for m in self.message_buffer {
            conn.message(m);
        }
        return conn;
    }
    pub fn message(&mut self, data: Arc<Json>) {
        self.message_buffer.push(data);
    }
}

impl Connection {
    pub fn message(&self, data: Arc<Json>) {
        self.channel.send(ConnectionMessage::Publish(data))
            .expect("send connection message");
    }
}
