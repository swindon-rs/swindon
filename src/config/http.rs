use std::str::FromStr;

use serde::de::{Deserializer, Deserialize};
use quire::validate::{Scalar};

use intern::Upstream;
use config::visitors::FromStrVisitor;


pub fn destination_validator() -> Scalar {
    Scalar::new()
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Destination {
    pub upstream: Upstream,
    pub path: String,
}

impl<'a> Deserialize<'a> for Destination {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(FromStrVisitor::new("upstream/path"))
    }
}

impl FromStr for Destination {
    type Err = String;
    fn from_str(val: &str) -> Result<Destination, String> {
        if let Some(path_start) = val.find('/') {
            Ok(Destination {
                upstream: Upstream::from_str(&val[..path_start])
                    .map_err(|e| format!("Invalid upstream: {}", e))?,
                path: val[path_start..].to_string(),
            })
        } else {
            Ok(Destination {
                upstream: Upstream::from_str(&val)
                    .map_err(|e| format!("Invalid upstream: {}", e))?,
                path: "/".to_string(),
            })
        }
    }
}
