use quire::validate::{Structure, Sequence, Scalar};


#[derive(RustcDecodable, PartialEq, Eq, Debug)]
pub struct Destination {
    pub addresses: Vec<String>,
}


pub fn destination_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("addresses", Sequence::new(Scalar::new()).min_length(1))
}
