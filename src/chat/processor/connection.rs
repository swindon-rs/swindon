use std::sync::Arc;
use std::collections::HashSet;

use rustc_serialize::json::Json;

use chat::Cid;
use intern::Atom;


pub struct Connection {
    pub cid: Cid,
    pub session_id: Atom,
    pub topics: HashSet<Atom>,
}

impl Connection {
    pub fn new(cid: Cid, session_id: Atom) -> Connection {
        Connection {
            cid: cid,
            session_id: session_id,
            topics: HashSet::new(),
        }
    }
    pub fn message(&self, data: Arc<Json>) {
        unimplemented!();
    }
}
