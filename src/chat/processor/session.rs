use std::sync::Arc;
use std::collections::{HashSet, HashMap};
use std::time::SystemTime;

use serde_json::Value as Json;

use chat::Cid;
use intern::{Lattice, SessionId};

pub struct UsersLattice {
    pub(in chat::processor) connections: HashSet<Cid>,
    pub(in chat::processor) peers: HashSet<SessionId>,
}

pub struct Session {
    pub(in chat::processor) status_timestamp: SystemTime,
    pub(in chat::processor) connections: HashSet<Cid>,
    pub(in chat::processor) lattices: HashMap<Lattice, HashSet<Cid>>,
    pub(in chat::processor) users_lattice: UsersLattice,
    pub(in chat::processor) metadata: Arc<Json>,
}

impl Session {
    pub fn new() -> Session {
        Session {
            status_timestamp: SystemTime::now(),
            connections: HashSet::new(),
            lattices: HashMap::new(),
            users_lattice: UsersLattice {
                connections: HashSet::new(),
                peers: HashSet::new(),
            },
            metadata: Arc::new(json!({})),
        }
    }
}
