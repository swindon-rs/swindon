use std::fmt;
use std::ops::Deref;
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

impl<'a> From<&'a str> for Atom {
    fn from(s: &'a str) -> Atom {
        if let Some(a) = ATOMS.read().expect("atoms locked").get(s) {
            return a.clone();
        }
        let newatom = Atom(Arc::new(String::from(s)));
        let mut atoms = ATOMS.write().expect("atoms locked");
        if !atoms.insert(newatom.clone()) {
            // Race condition happened, but now we are still holding lock
            // so it's safe to unwrap
            return atoms.get(s).unwrap().clone();
        } else {
            return newatom;
        }
    }
}
impl From<String> for Atom {
    fn from(s: String) -> Atom {
        if let Some(a) = ATOMS.read().expect("atoms locked").get(&s) {
            return a.clone();
        }
        let newatom = Atom(Arc::new(s));
        let mut atoms = ATOMS.write().expect("atoms locked");
        if !atoms.insert(newatom.clone()) {
            // Race condition happened, but now we are still holding lock
            // so it's safe to unwrap
            return atoms.get(&newatom).unwrap().clone();
        } else {
            return newatom;
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
        d.read_str().map(Atom::from)
    }
}

impl Deref for Atom {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
