use std::sync::Arc;
use std::collections::{BTreeMap, HashSet};

use rustc_serialize::json::Json;

use chat::Cid;


pub struct Session {
    pub connections: HashSet<Cid>,
    pub metadata: Arc<Json>,
}

impl Session {
    pub fn new() -> Session {
        Session {
            connections: HashSet::new(),
            metadata: Arc::new(Json::Object(BTreeMap::new())),
        }
    }
}
