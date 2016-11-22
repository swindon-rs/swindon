use std::collections::{HashMap, HashSet};

use intern::LatticeKey as Key;

pub struct Counter(u64);
pub struct Set(HashSet<String>);

pub struct Values {
    counters: HashMap<Key, Counter>,
    sets: HashMap<Key, Set>,
}
