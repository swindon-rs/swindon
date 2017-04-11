use std::sync::Arc;
use std::ascii::AsciiExt;
use std::path::PathBuf;
use std::collections::HashMap;

use quire::validate::{Nothing, Enum, Structure, Scalar, Mapping, Sequence};
use rustc_serialize::{Decoder, Decodable};

use intern::DiskPoolName;


#[derive(RustcDecodable, Debug, PartialEq, Eq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Mode {
    relative_to_domain_root,
    relative_to_route,
    with_hostname,
}

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum VersionChars {
    lowercase_hex,
}

#[derive(RustcDecodable, Debug, PartialEq, Eq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FallbackMode {
    always,
    no_file,      // file not found
    bad_version,  // when value in version-arg argument is invalid
    no_version,   // when no version-arg is specified
    never,        // don't serve anything without valid version
}

#[derive(Debug, PartialEq, Eq)]
pub struct Static {
    pub mode: Mode,
    pub path: PathBuf,
    pub text_charset: Option<String>,
    pub pool: DiskPoolName,
    pub extra_headers: HashMap<String, String>,
    pub strip_host_suffix: Option<String>,
    pub index_files: Vec<String>,
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

#[derive(Debug, PartialEq, Eq)]
pub struct VersionedStatic {
    pub versioned_root: PathBuf,
    pub plain_root: PathBuf,
    pub version_arg: String,
    pub version_split: Vec<u32>,
    pub version_chars: VersionChars,
    pub fallback_to_plain: FallbackMode,
    pub fallback_mode: Mode,
    pub text_charset: Option<String>,
    pub pool: DiskPoolName,
    pub extra_headers: HashMap<String, String>,
    // Computed values
    pub version_len: usize,
    pub overrides_content_type: bool,
    pub fallback: Arc<Static>,
}

fn serve_mode<'x>() -> Enum<'x> {
    Enum::new()
        .option("relative_to_domain_root", Nothing)
        .option("relative_to_route", Nothing)
        .option("with_hostname", Nothing)
        .allow_plain()
        .plain_default("relative_to_route")
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("mode", serve_mode())
    .member("path", Scalar::new())
    .member("text_charset", Scalar::new().optional())
    .member("pool", Scalar::new().default("default"))
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
    .member("strip_host_suffix", Scalar::new().optional())
    .member("index_files", Sequence::new(Scalar::new()))
}

pub fn single_file<'x>() -> Structure<'x> {
    Structure::new()
    .member("path", Scalar::new())
    .member("content_type", Scalar::new())
    .member("pool", Scalar::new().default("default"))
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
}

pub fn versioned_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("versioned_root", Scalar::new())
    .member("plain_root", Scalar::new().optional())
    .member("version_arg", Scalar::new())
    .member("version_split", Sequence::new(Scalar::new()))
    .member("version_chars", Enum::new()
        .option("lowercase_hex", Nothing)
        .allow_plain())
    .member("fallback_to_plain", Enum::new()
        .option("always", Nothing)
        .option("no_file", Nothing)
        .option("bad_version", Nothing)
        .option("no_version", Nothing)
        .option("never", Nothing)
        .allow_plain()
        .plain_default("never"))
    .member("fallback_mode", serve_mode())
    .member("text_charset", Scalar::new().optional())
    .member("pool", Scalar::new().default("default"))
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
    .member("strip_host_suffix", Scalar::new().optional())
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
            pub index_files: Vec<String>,
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
            index_files: int.index_files,
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

impl Decodable for VersionedStatic {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        #[derive(RustcDecodable)]
        pub struct Internal {
            pub versioned_root: PathBuf,
            pub plain_root: PathBuf,
            pub version_arg: String,
            pub version_split: Vec<u32>,
            pub version_chars: VersionChars,
            pub fallback_to_plain: FallbackMode,
            pub fallback_mode: Mode,
            pub text_charset: Option<String>,
            pub pool: DiskPoolName,
            pub extra_headers: HashMap<String, String>,
        }
        let int = Internal::decode(d)?;
        return Ok(VersionedStatic {
            version_len: int.version_split.iter().map(|&x| x as usize).sum(),
            overrides_content_type:
                header_contains(&int.extra_headers, "Content-Type"),
            fallback: Arc::new(Static {
                overrides_content_type:
                    header_contains(&int.extra_headers, "Content-Type"),
                mode: int.fallback_mode.clone(),
                path: int.plain_root.clone(),
                text_charset: int.text_charset.clone(),
                pool: int.pool.clone(),
                extra_headers: int.extra_headers.clone(),
                index_files: Vec::new(),
                strip_host_suffix: None,
            }),
            versioned_root: int.versioned_root,
            plain_root: int.plain_root,
            version_arg: int.version_arg,
            version_split: int.version_split,
            version_chars: int.version_chars,
            fallback_to_plain: int.fallback_to_plain,
            fallback_mode: int.fallback_mode,
            text_charset: int.text_charset,
            pool: int.pool,
            extra_headers: int.extra_headers,
        })
    }
}

pub fn header_contains(map: &HashMap<String, String>, name: &str) -> bool {
    map.iter().any(|(header, _)| header.eq_ignore_ascii_case(name))
}
