use std::path::PathBuf;

use quire::validate::{Nothing, Enum, Structure, Scalar};


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Mode {
    relative_to_site_root,
    relative_to_route,
}


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Static {
    pub mode: Mode,
    pub path: PathBuf,
    pub text_charset: Option<String>,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("mode", Enum::new()
        .option("relative_to_site_root", Nothing)
        .option("relative_to_route", Nothing)
        .allow_plain())
    .member("path", Scalar::new())
    .member("text_charset", Scalar::new().optional())
}
