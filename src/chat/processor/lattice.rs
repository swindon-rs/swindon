use std::sync::Arc;
use std::hash::Hash;
use std::fmt;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry::{Occupied, Vacant};

use serde::ser::{Serialize, Serializer, SerializeMap};
use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess};

use intern::{LatticeKey as Key, LatticeVar as Var, SessionId};
use metrics::{Integer};

lazy_static! {
    pub static ref SHARED_KEYS: Integer = Integer::new();
    pub static ref SHARED_COUNTERS: Integer = Integer::new();
    pub static ref SHARED_SETS: Integer = Integer::new();
    pub static ref PRIVATE_KEYS: Integer = Integer::new();
    pub static ref PRIVATE_COUNTERS: Integer = Integer::new();
    pub static ref PRIVATE_SETS: Integer = Integer::new();
    pub static ref SET_ITEMS: Integer = Integer::new();
}


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
    counters: HashMap<Var, Counter>,
    sets: HashMap<Var, Set>,
}

pub struct Lattice {
    pub shared: HashMap<Key, Values>,
    pub private: HashMap<SessionId, HashMap<Key, Values>>,
    pub subscriptions: HashMap<Key, HashSet<SessionId>>,
}

#[derive(Debug, Clone, Serialize)]
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

impl Serialize for Values {
    fn serialize<S: Serializer>(&self, serialize: S)
        -> Result<S::Ok, S::Error>
    {
        let mut map = serialize.serialize_map(
            Some(self.counters.len() + self.sets.len()))?;
        for (k, counter) in &self.counters {
            map.serialize_key(&format!("{}_counter", k))?;
            map.serialize_value(&counter.0)?;
        }
        for (k, set) in &self.sets {
            map.serialize_key(&format!("{}_set", k))?;
            map.serialize_value(&set.0)?;
        }
        map.end()
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
                        .or_insert_with(|| {
                            SHARED_KEYS.incr(1);
                            Values::new()
                        });

            crdt_update(&mut mine.counters, &mut values.counters,
                &*SHARED_COUNTERS);
            crdt_update(&mut mine.sets, &mut values.sets,
                &*SHARED_SETS);

            if values.is_empty() {
                del.push(room.clone());
            }
        }
        for key in &del {
            if delta.shared.remove(key).is_some() {
              SHARED_KEYS.decr(1);
            }
        }

        for (session_id, rooms) in &mut delta.private {
            let mysess = self.private.entry(session_id.clone())
                .or_insert_with(HashMap::new);
            let mut del_rooms = Vec::new();
            for (room, values) in rooms.iter_mut() {
                let mine = mysess.entry(room.clone())
                    .or_insert_with(|| {
                        PRIVATE_KEYS.incr(1);
                        Values::new()
                    });

                crdt_update(&mut mine.counters, &mut values.counters,
                    &*PRIVATE_COUNTERS);
                crdt_update(&mut mine.sets, &mut values.sets,
                    &*PRIVATE_SETS);

                if values.is_empty() {
                    del_rooms.push(room.clone());
                }
            }
            for key in &del_rooms {
                if rooms.remove(key).is_some() {
                    PRIVATE_KEYS.decr(1);
                }
            }
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
                if nset.insert(key.clone()) {
                    SET_ITEMS.incr(1);
                }
                for item in iter {
                    if nset.insert(item.clone()) {
                        SET_ITEMS.incr(1);
                    }
                }
                return true;
            }
        }
        return false;
    }
}

fn crdt_update<K, V>(original: &mut HashMap<K, V>, delta: &mut HashMap<K, V>,
    number: &Integer)
    where K: Clone + Hash + Eq + ::std::fmt::Debug, V: Crdt
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
                number.incr(1);
                entry.insert(crdt.clone());
            }
        }
    }
    for key in &del {
        if original.remove(key).is_some() {
            number.decr(1);
        }
    }
}

impl<'de> Deserialize<'de> for Delta {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        struct DeltaOpt {
            pub shared: Option<HashMap<Key, Values>>,
            pub private: Option<HashMap<SessionId, HashMap<Key, Values>>>,
        }
        let tmp = DeltaOpt::deserialize(deserializer)?;
        Ok(Delta {
            shared: tmp.shared.unwrap_or_else(HashMap::new),
            private: tmp.private.unwrap_or_else(HashMap::new),
        })
    }
}


struct ValuesVisitor;

impl<'de> Visitor<'de> for ValuesVisitor {
    type Value = Values;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("map expected")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where M: MapAccess<'de>
    {
        let mut values = Values {
            counters: HashMap::new(),
            sets: HashMap::new(),
        };
        while let Some(key) = access.next_key::<&str>()? {
            if key.ends_with("_counter") {
                let val = access.next_value()?;
                let key = key[..key.len() - "_counter".len()].parse()
                    .map_err(de::Error::custom)?;
                values.counters.insert(key, Counter(val));
            } else if key.ends_with("_set") {
                let val = access.next_value()?;
                let key = key[..key.len() - "_set".len()].parse()
                    .map_err(de::Error::custom)?;
                values.sets.insert(key, val);
            } else {
                return Err(de::Error::custom(format!(
                    "Unsupported key {:?}", key)))
            }
        }
        Ok(values)
    }
}

impl<'de> Deserialize<'de> for Values {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_map(ValuesVisitor)
    }
}

impl<'de> Deserialize<'de> for Set {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        Ok(Set(Deserialize::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;
    use serde_json::from_str as json_decode;

    use super::{Delta, Values, Set};
    use intern::{LatticeKey as Key, LatticeVar as Var};

    #[test]
    fn decode_delta() {
        let val = r#"{"shared": {"room_1": {"last_message_counter": 125}},
            "private": {"user:1": {"room_1": {
                "last_seen_set": ["123", "124"]
            }}}}"#;

        let delta: Delta = json_decode(val).unwrap();
        assert_eq!(delta.shared.len(), 1);
        assert_eq!(delta.private.len(), 1);
        assert!(delta.shared.contains_key(&Key::from_str("room_1").unwrap()));
    }

    #[test]
    fn decode_values() {
        let val = r#"{"last_message_counter": 123}"#;
        let val: Values = json_decode(val).unwrap();
        assert_eq!(val.counters.len(), 1);
        assert_eq!(val.sets.len(), 0);

        let key = Var::from_str("last_message").unwrap();
        assert!(val.counters.contains_key(&key));
        assert_eq!(val.counters.get(&key).unwrap().0, 123u64);
    }

    #[test]
    fn decode_set() {
        let set: Set = json_decode(r#"["123", "123", "abc"]"#).unwrap();
        assert_eq!(set.0.len(), 2);
    }
}
