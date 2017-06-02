use std::sync::Arc;
use std::collections::{HashSet, HashMap};

use serde_json::Value as Json;

use chat::Cid;
use intern::Lattice;


pub struct Session {
    pub connections: HashSet<Cid>,
    pub lattices: HashMap<Lattice, HashSet<Cid>>,
    pub metadata: Arc<Json>,
}

impl Session {
    pub fn new() -> Session {
        Session {
            connections: HashSet::new(),
            lattices: HashMap::new(),
            metadata: Arc::new(json!({})),
        }
    }
}
