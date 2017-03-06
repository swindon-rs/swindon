use std::ascii::AsciiExt;
use std::path::PathBuf;
use std::collections::HashMap;

use quire::validate::{Nothing, Enum, Structure, Scalar, Mapping};
use rustc_serialize::{Decoder, Decodable};

use intern::DiskPoolName;


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Mode {
    relative_to_domain_root,
    relative_to_route,
    with_hostname,
}


#[derive(Debug, PartialEq, Eq)]
pub struct Static {
    pub mode: Mode,
    pub path: PathBuf,
    pub text_charset: Option<String>,
    pub pool: DiskPoolName,
    pub extra_headers: HashMap<String, String>,
    pub strip_host_suffix: Option<String>,
    // Computed values
    pub overrides_content_type: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SingleFile {
    pub path: PathBuf,
    pub content_type: String,
    pub pool: DiskPoolName,
    pub extra_headers: HashMap<String, String>,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("mode", Enum::new()
        .option("relative_to_domain_root", Nothing)
        .option("relative_to_route", Nothing)
        .option("with_hostname", Nothing)
        .allow_plain()
        .plain_default("relative_to_route"))
    .member("path", Scalar::new())
    .member("text_charset", Scalar::new().optional())
    .member("pool", Scalar::new().default("default"))
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
    .member("strip_host_suffix", Scalar::new().optional())
}

pub fn single_file<'x>() -> Structure<'x> {
    Structure::new()
    .member("path", Scalar::new())
    .member("content_type", Scalar::new())
    .member("pool", Scalar::new().default("default"))
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
}

impl Decodable for Static {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        #[derive(RustcDecodable)]
        pub struct Internal {
            pub mode: Mode,
            pub path: PathBuf,
            pub text_charset: Option<String>,
            pub pool: DiskPoolName,
            pub extra_headers: HashMap<String, String>,
            pub strip_host_suffix: Option<String>,
        }
        let int = Internal::decode(d)?;
        return Ok(Static {
            overrides_content_type:
                header_contains(&int.extra_headers, "Content-Type"),
            mode: int.mode,
            path: int.path,
            text_charset: int.text_charset,
            pool: int.pool,
            extra_headers: int.extra_headers,
            strip_host_suffix: int.strip_host_suffix,
        })
    }
}

impl Decodable for SingleFile {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        #[derive(RustcDecodable)]
        pub struct Internal {
            pub path: PathBuf,
            pub content_type: String,
            pub pool: DiskPoolName,
            pub extra_headers: HashMap<String, String>,
        }
        let int = Internal::decode(d)?;
        if header_contains(&int.extra_headers, "Content-Type") {
            return Err(d.error("Content-Type must be specified as \
                `content-type` parameter rather than in `extra-headers` \
                in `!SingleFile` handler."));
        }
        return Ok(SingleFile {
            path: int.path,
            content_type: int.content_type,
            pool: int.pool,
            extra_headers: int.extra_headers,
        })
    }
}

pub fn header_contains(map: &HashMap<String, String>, name: &str) -> bool {
    map.iter().any(|(header, _)| header.eq_ignore_ascii_case(name))
}
