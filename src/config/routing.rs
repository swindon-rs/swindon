use std::str::FromStr;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::cmp::{Ordering, PartialOrd, Ord};

use intern::HandlerName;
use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Mapping, Scalar};
use routing::RoutingTable;



pub type Routing = RoutingTable<HandlerName>;

pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}

