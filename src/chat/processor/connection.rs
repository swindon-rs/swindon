use std::sync::Arc;
use std::collections::HashSet;

use rustc_serialize::json::Json;

use chat::Cid;
use intern::Atom;


pub struct NewConnection {
    pub cid: Cid,
    pub topics: HashSet<Atom>,
    pub message_buffer: Vec<Arc<Json>>,
}


pub struct Connection {
    pub cid: Cid,
    pub session_id: Atom,
    pub topics: HashSet<Atom>,
}

impl NewConnection {
    pub fn new(conn_id: Cid) -> NewConnection {
        NewConnection {
            cid: conn_id,
            topics: HashSet::new(),
            message_buffer: Vec::new(),
        }
    }
    pub fn associate(self, session_id: Atom) -> Connection {
        let conn = Connection {
            cid: self.cid,
            session_id: session_id,
            topics: self.topics,
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
        unimplemented!();
    }
}
