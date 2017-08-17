use std::collections::HashMap;

use quire::validate::{Structure, Mapping, Scalar};
use config::static_files::header_contains;
use serde::de::{Deserializer, Deserialize};


#[derive(Debug, PartialEq, Eq)]
pub struct EmptyGif {
    pub extra_headers: HashMap<String, String>,
    // Computed values
    pub overrides_content_type: bool,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
}

impl<'a> Deserialize<'a> for EmptyGif {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        pub struct Internal {
            pub extra_headers: HashMap<String, String>,
        }
        let int = Internal::deserialize(d)?;
        return Ok(EmptyGif {
            overrides_content_type:
                header_contains(&int.extra_headers, "Content-Type"),
            extra_headers: int.extra_headers,
        })
    }
}
