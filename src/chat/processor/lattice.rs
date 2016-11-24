use std::sync::Arc;
use std::hash::Hash;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry::{Occupied, Vacant};

use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};

use intern::{LatticeKey as Key, SessionId};

#[derive(Debug, Clone)]
pub struct Counter(u64);
// TODO(tailhook) implement some persistent hash set
// TODO(tailhook) optimize set of only int-like values
#[derive(Debug, Clone)]
pub struct Set(Arc<HashSet<String>>);

trait Crdt: Clone + Sized {
    /// Updates returning `true` if value changed
    fn update(&mut self, other: &Self) -> bool;
}

#[derive(Debug, Clone)]
pub struct Values {
    counters: HashMap<Key, Counter>,
    sets: HashMap<Key, Set>,
}

pub struct Lattice {
    pub shared: HashMap<Key, Values>,
    pub private: HashMap<SessionId, HashMap<Key, Values>>,
    pub subscriptions: HashMap<Key, HashSet<SessionId>>,
}

pub struct Delta {
    pub shared: HashMap<Key, Values>,
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
    pub fn is_empty(&self) -> bool {
        self.counters.len() == 0 && self.sets.len() == 0
    }
}

impl Encodable for Values {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error>
    {
        s.emit_map(self.counters.len() + self.sets.len(), |s| {
            let mut i = 0;
            for (k, counter) in &self.counters {
                s.emit_map_elt_key(i, |s| format!("{}_counter", k).encode(s))?;
                s.emit_map_elt_val(i, |s| s.emit_u64(counter.0))?;
                i += 1;
            }
            for (k, set) in &self.sets {
                s.emit_map_elt_key(i, |s| format!("{}_set", k).encode(s))?;
                s.emit_map_elt_val(i, |s| set.0.encode(s))?;
                i += 1;
            }
            Ok(())
        })
    }
}

impl Lattice {
    pub fn new() -> Lattice {
        Lattice {
            shared: HashMap::new(),
            private: HashMap::new(),
            subscriptions: HashMap::new(),
        }
    }
    /// Updates lattice to be up to date with Delta and returns modified delta
    /// that contains only data that really changed and not out of date
    pub fn update(&mut self, mut delta: Delta) -> Delta {
        let mut del = Vec::new();
        for (room, values) in &mut delta.shared {
            let mine = self.shared.entry(room.clone())
                        .or_insert_with(Values::new);

            crdt_update(&mut mine.counters, &mut values.counters);
            crdt_update(&mut mine.sets, &mut values.sets);

            if values.is_empty() {
                del.push(room.clone());
            }
        }
        for key in &del {
            delta.shared.remove(key);
        }

        let mut del = Vec::new();
        for (session_id, rooms) in &mut delta.private {
            let mysess =
                if let Some(s) = self.private.get_mut(session_id) {
                    s
                } else {
                    // There is no such session, don't need to store data for
                    // it
                    //
                    // Note:
                    // * we remove non-existent sessions
                    // * but keep sessions with empty rooms in the delta
                    //
                    // This is intentional! Refer to the protocol
                    // documentation for more information.
                    del.push(session_id.clone());
                    continue;
                };
            let mut del_rooms = Vec::new();
            for (room, values) in rooms.iter_mut() {
                let mine = mysess.entry(room.clone())
                    .or_insert_with(Values::new);

                crdt_update(&mut mine.counters, &mut values.counters);
                crdt_update(&mut mine.sets, &mut values.sets);

                if values.is_empty() {
                    del_rooms.push(room.clone());
                }
            }
            for key in &del_rooms {
                rooms.remove(key);
            }
        }
        for key in &del {
            delta.private.remove(key);
        }
        return delta
    }
}

impl Crdt for Counter {
    fn update(&mut self, other: &Self) -> bool {
        if self.0 < other.0 {
            self.0 = other.0;
            true
        } else {
            false
        }
    }
}

impl Crdt for Set {
    fn update(&mut self, other: &Self) -> bool {
        let mut iter = other.0.iter();
        let ref mut arc = self.0;
        while let Some(key) = iter.next() {
            if !arc.contains(key) {
                let nset = Arc::make_mut(arc);
                nset.insert(key.clone());
                for item in iter {
                    nset.insert(item.clone());
                }
                return true;
            }
        }
        return false;
    }
}

fn crdt_update<K, V>(original: &mut HashMap<K, V>, delta: &mut HashMap<K, V>)
    where K: Clone + Hash + Eq, V: Crdt
{
    let mut del = Vec::new();
    for (key, crdt) in delta {
        match original.entry(key.clone()) {
            Occupied(mut entry) => {
                if !entry.get_mut().update(crdt) {
                    del.push(key.clone());
                }
            }
            Vacant(entry) => {
                entry.insert(crdt.clone());
            }
        }
    }
    for key in &del {
        original.remove(key);
    }
}

impl Decodable for Delta {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error>
    {
        d.read_map(|d, size| {
            if size > 2 {
                return Err(d.error(
                    format!("expected at most 2 elements, got {:?}", size).as_str()))
            }
            let mut shared = HashMap::new();
            let mut private = HashMap::new();
            let mut prevkey = "";
            for idx in 0..size {
                let key = d.read_map_elt_key(idx, |d| d.read_str())?;
                prevkey = match (key.as_str(), prevkey) {
                    ("shared", "") | ("shared", "private") => {
                        shared = d.read_map_elt_val(idx, |d| {
                            HashMap::<Key, Values>::decode(d)
                        })?;
                        "shared"
                    }
                    ("private", "") | ("private", "shared") => {
                        private = d.read_map_elt_val(idx, |d| {
                            HashMap::<SessionId, HashMap<Key, Values>>::decode(d)
                        })?;
                        "private"
                    }
                    (other, _) => {
                        return Err(d.error(
                            format!("unexpected key {}", other).as_str()))
                    }
                };
            }
            Ok(Delta {
                shared: shared,
                private: private,
            })
        })
    }
}

impl Decodable for Values {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error>
    {
        let mut values = Values {
            counters: HashMap::new(),
            sets: HashMap::new(),
        };
        d.read_map(|d, size| {
            for idx in 0..size {
                let key = d.read_map_elt_key(idx, |d| Key::decode(d))?;
                if key.as_ref().ends_with("_counter") {
                    let val = d.read_map_elt_val(idx, |d| d.read_u64())?;
                    values.counters.insert(key, Counter(val));
                } else if key.as_ref().ends_with("_set") {
                    let val = d.read_map_elt_val(idx, |d| Set::decode(d))?;
                    values.sets.insert(key, val);
                } else {
                    return Err(d.error(format!(
                        "Unsupported key {:?}", key).as_str()))
                }
            }
            Ok(values)
        })
    }
}

impl Decodable for Set {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error>
    {
        d.read_seq(|d, size| {
            let mut set = HashSet::new();
            for idx in 0..size {
                set.insert(d.read_seq_elt(idx, |d| d.read_str())?);
            }
            Ok(Set(Arc::new(set)))
        })
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;
    use rustc_serialize::json;

    use super::{Delta, Values, Set};
    use intern::LatticeKey as Key;

    #[test]
    fn decode_delta() {
        let val = r#"{"shared": {"room_1": {"last_message_counter": 125}},
            "private": {"user:1": {"room_1": {
                "last_seen_set": ["123", "124"]
            }}}}"#;

        let delta: Delta = json::decode(val).unwrap();
        assert_eq!(delta.shared.len(), 1);
        assert_eq!(delta.private.len(), 1);
        assert!(delta.shared.contains_key(&Key::from_str("room_1").unwrap()));
    }

    #[test]
    fn decode_values() {
        let val = r#"{"last_message_counter": 123}"#;
        let val: Values = json::decode(val).unwrap();
        assert_eq!(val.counters.len(), 1);
        assert_eq!(val.sets.len(), 0);

        let key = Key::from_str("last_message_counter").unwrap();
        assert!(val.counters.contains_key(&key));
        assert_eq!(val.counters.get(&key).unwrap().0, 123u64);
    }

    #[test]
    fn decode_set() {
        let set: Set = json::decode(r#"["123", "123", "abc"]"#).unwrap();
        assert_eq!(set.0.len(), 2);
    }
}
