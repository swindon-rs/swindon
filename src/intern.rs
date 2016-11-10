use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::borrow::Borrow;
use std::sync::{Arc, RwLock};
use std::collections::HashSet;

use rustc_serialize::{Decoder, Decodable};

lazy_static! {
    static ref ATOMS: RwLock<HashSet<Atom>> = RwLock::new(HashSet::new());
}

// TODO(tailhook) optimize Eq to compare pointers
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Atom(Arc<String>);

quick_error! {
    #[derive(Debug)]
    pub enum InvalidAtom {
        InvalidChar {
            description("invalid character in atom")
        }
    }
}

fn is_valid(val: &str) -> bool {
    val.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

impl FromStr for Atom {
    type Err = InvalidAtom;
    fn from_str(s: &str) -> Result<Atom, InvalidAtom> {
        if let Some(a) = ATOMS.read().expect("atoms locked").get(s) {
            return Ok(a.clone());
        }
        if !is_valid(s) {
            return Err(InvalidAtom::InvalidChar);
        }
        let newatom = Atom(Arc::new(String::from(s)));
        let mut atoms = ATOMS.write().expect("atoms locked");
        if !atoms.insert(newatom.clone()) {
            // Race condition happened, but now we are still holding lock
            // so it's safe to unwrap
            return Ok(atoms.get(s).unwrap().clone());
        } else {
            return Ok(newatom);
        }
    }
}

impl Borrow<str> for Atom {
    fn borrow(&self) -> &str {
        &self.0[..]
    }
}

impl Borrow<String> for Atom {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a{:?}", self.0)
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl Decodable for Atom {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        use std::error::Error;
        d.read_str()?
        .parse::<Atom>()
        .map_err(|e| d.error(e.description()))
    }
}

impl Deref for Atom {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl Atom {
    pub fn from(s: &'static str) -> Atom {
        FromStr::from_str(s)
        .expect("static strings used as atom is invalid")
    }
}
