use std::collections::HashMap;

use quire::validate::{Structure, Mapping, Scalar};
use config::static_files::header_contains;
use rustc_serialize::{Decoder, Decodable};


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

impl Decodable for EmptyGif {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        #[derive(RustcDecodable)]
        pub struct Internal {
            pub extra_headers: HashMap<String, String>,
        }
        let int = Internal::decode(d)?;
        return Ok(EmptyGif {
            overrides_content_type:
                header_contains(&int.extra_headers, "Content-Type"),
            extra_headers: int.extra_headers,
        })
    }
}
