use std::sync::Arc;
use std::collections::{HashMap, HashSet};

use rustc_serialize::{Encodable, Encoder};

use intern::{LatticeKey as Key, Lattice as Namespace, SessionId};

#[derive(Debug, Clone)]
pub struct Counter(u64);
// TODO(tailhook) implement some persistent hash set
// TODO(tailhook) optimize set of only int-like values
#[derive(Debug, Clone)]
pub struct Set(Arc<HashSet<String>>);

#[derive(Debug, Clone)]
pub struct Values {
    counters: HashMap<Key, Counter>,
    sets: HashMap<Key, Set>,
}

pub struct Lattice {
    pub public: HashMap<Key, Values>,
    pub private: HashMap<SessionId, HashMap<Key, Values>>,
}

// It look like the same as lattice, but we consider it a different type
// so we can add more state (cache) in Lattice later
pub struct Delta {
    pub public: HashMap<Key, Values>,
    pub private: HashMap<SessionId, HashMap<Key, Values>>,
}

impl Values {
    pub fn new() -> Values {
        Values {
            counters: HashMap::new(),
            sets: HashMap::new(),
        }
    }
    pub fn update(&mut self, other: &Values) {
        for (key, value) in &other.counters {
            self.counters.insert(key.clone(), value.clone());
        }
        for (key, value) in &other.sets {
            self.sets.insert(key.clone(), value.clone());
        }
    }
}

impl Encodable for Values {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error>
    {
        s.emit_map(self.counters.len() + self.sets.len(), |s| {
            let mut i = 0;
            for (k, counter) in &self.counters {
                s.emit_map_elt_key(i, |s| format!("{}_counter", k).encode(s));
                s.emit_map_elt_val(i, |s| s.emit_u64(counter.0));
                i += 1;
            }
            for (k, set) in &self.sets {
                s.emit_map_elt_key(i, |s| format!("{}_set", k).encode(s));
                s.emit_map_elt_val(i, |s| set.0.encode(s));
                i += 1;
            }
            Ok(())
        })
    }
}
