use std::collections::HashMap;

use quire::validate::{Structure, Mapping, Scalar};

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct EmptyGif {
    pub extra_headers: HashMap<String, String>,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
}
