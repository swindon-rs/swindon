use intern::HandlerName;
use routing::RoutingTable;

use quire::validate::{Mapping, Scalar};


pub type Routing = RoutingTable<HandlerName>;

pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}

