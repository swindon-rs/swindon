use std::path::PathBuf;

use quire::validate::{Structure, Sequence, Scalar};


#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ClientSettings {
    pub certificates: Vec<PathBuf>,
}

pub fn client_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("certificates", Sequence::new(Scalar::new()))
}
