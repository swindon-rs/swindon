use quire::validate::{Mapping, Scalar};

use intern::Authorizer;
use routing::RoutingTable;


pub type Authorization = RoutingTable<Authorizer>;


pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}
