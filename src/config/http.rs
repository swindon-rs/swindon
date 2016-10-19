use std::str::FromStr;

use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Scalar};

use intern::Atom;


pub fn destination_validator() -> Scalar {
    Scalar::new()
}

#[derive(PartialEq, Eq, Debug)]
pub struct Destination {
    pub upstream: Atom,
    pub path: String,
}

impl Decodable for Destination {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        try!(d.read_str())
        .parse()
        .map_err(|e: String| d.error(&e))
    }
}

impl FromStr for Destination {
    type Err = String;
    fn from_str(val: &str) -> Result<Destination, String> {
        if let Some(path_start) = val.find('/') {
            Ok(Destination {
                upstream: Atom::from(&val[..path_start]),
                path: val[path_start..].to_string(),
            })
        } else {
            Ok(Destination {
                upstream: Atom::from(val),
                path: "/".to_string(),
            })
        }
    }
}
