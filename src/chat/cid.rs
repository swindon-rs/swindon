use std::fmt;
use std::str::FromStr;
use std::num::ParseIntError;
use serde::de::{self, Deserialize, Deserializer, Visitor};

use crate::runtime::ServerId;

/// Internal connection id
#[derive(Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Cid(u64);


/// Public connection id
pub struct PubCid(pub Cid, pub ServerId);

impl Cid {
    pub fn new() -> Cid {
        // Until atomic u64 really works
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Cid(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}


impl FromStr for Cid {
    type Err = ParseIntError;

    fn from_str(src: &str) -> Result<Cid, Self::Err> {
        src.parse().map(|x| Cid(x))
    }
}

impl fmt::Debug for Cid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "cid:{}", self.0)
        } else {
            write!(f, "Cid({})", self.0)
        }
    }
}

impl fmt::Display for Cid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PubCid {
    type Err = ();

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let s = src.rfind('-').ok_or(())?;
        let (rid, cid) = src.split_at(s);
        let rid = rid.parse().map_err(|_| ())?;
        let cid = cid[1..].parse().map_err(|_| ())?;
        Ok(PubCid(cid, rid))
    }
}

impl<'de> Deserialize<'de> for PubCid {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        d.deserialize_str(CidVisitor)
    }
}

struct CidVisitor;

impl<'de> Visitor<'de> for CidVisitor {
    type Value = PubCid;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "valid connection id string")
    }
    fn visit_str<E>(self, val: &str) -> Result<Self::Value, E>
        where E: de::Error
    {
        val.parse().map_err(|_| de::Error::custom("invalid connection id"))
    }
}
