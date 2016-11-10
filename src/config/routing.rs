use std::str::FromStr;
use std::collections::BTreeMap;

use intern::Atom;
use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Mapping, Scalar};


#[derive(Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct Route {
    pub host: String,
    pub path: Option<String>,
}

pub type Routing = BTreeMap<Route, Atom>;

pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}

impl Decodable for Route {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_str()?
        .parse()
        .map_err(|e: String| d.error(&e))
    }
}

impl FromStr for Route {
    type Err = String;
    fn from_str(val: &str) -> Result<Route, String> {
        if let Some(path_start) = val.find('/') {
            Ok(Route {
                host: val[..path_start].to_string(),
                path: Some(val[path_start..].to_string()),
            })
        } else {
            Ok(Route {
                host: val.to_string(),
                path: None,
            })
        }
    }
}
