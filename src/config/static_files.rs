use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;

use http_file_headers::{Config as HeadersConfig};
use quire::validate::{Nothing, Enum, Structure, Scalar, Mapping, Sequence};
use quire::validate::{Numeric};
use serde::de::{Deserializer, Deserialize, Error};

use intern::DiskPoolName;


#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Mode {
    relative_to_domain_root,
    relative_to_route,
    with_hostname,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum VersionChars {
    lowercase_hex,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FallbackMode {
    always,
    no_file,      // file not found
    bad_version,  // when value in version-arg argument is invalid
    no_version,   // when no version-arg is specified
    never,        // don't serve anything without valid version
}

#[derive(Debug)]
pub struct Static {
    pub mode: Mode,
    pub path: PathBuf,
    pub text_charset: Option<String>,
    pub pool: DiskPoolName,
    pub extra_headers: HashMap<String, String>,
    pub strip_host_suffix: Option<String>,
    pub index_files: Vec<String>,
    pub generate_index: bool,
    pub generated_index_max_files: usize,
    // Computed values
    pub headers_config: Arc<HeadersConfig>,
}

#[derive(Debug)]
pub struct SingleFile {
    pub path: PathBuf,
    pub content_type: Option<String>,
    pub pool: DiskPoolName,
    pub extra_headers: HashMap<String, String>,
    // Computed values
    pub headers_config: Arc<HeadersConfig>,
}

#[derive(Debug)]
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
    pub fallback: Arc<Static>,
    pub headers_config: Arc<HeadersConfig>,
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
    .member("text_charset", Scalar::new().default("utf-8").optional())
    .member("pool", Scalar::new().default("default"))
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
    .member("strip_host_suffix", Scalar::new().optional())
    .member("index_files", Sequence::new(Scalar::new()))
    .member("generate_index", Scalar::new().default(false))
    .member("generated_index_max_files",
        Numeric::new().min(0).default(100000))
}

pub fn single_file<'x>() -> Structure<'x> {
    Structure::new()
    .member("path", Scalar::new())
    .member("content_type", Scalar::new().optional())
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
    .member("text_charset", Scalar::new().default("utf-8").optional())
    .member("pool", Scalar::new().default("default"))
    .member("extra_headers", Mapping::new(Scalar::new(), Scalar::new()))
    .member("strip_host_suffix", Scalar::new().optional())
}

impl<'a> Deserialize<'a> for Static {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        pub struct Internal {
            pub mode: Mode,
            pub path: PathBuf,
            pub text_charset: Option<String>,
            pub pool: DiskPoolName,
            pub extra_headers: HashMap<String, String>,
            pub index_files: Vec<String>,
            pub generate_index: bool,
            pub generated_index_max_files: usize,
            pub strip_host_suffix: Option<String>,
        }
        let int = Internal::deserialize(d)?;
        let mut config = HeadersConfig::new();
        match int.text_charset {
            Some(ref charset) => { config.text_charset(charset); }
            None => { config.no_text_charset(); }
        }
        if header_contains(&int.extra_headers, "Content-Type") {
            config.content_type(false);
        }
        for index_file in &int.index_files {
            config.add_index_file(&index_file);
        }
        return Ok(Static {
            mode: int.mode,
            path: int.path,
            text_charset: int.text_charset,
            pool: int.pool,
            extra_headers: int.extra_headers,
            index_files: int.index_files,
            generate_index: int.generate_index,
            generated_index_max_files: int.generated_index_max_files,
            strip_host_suffix: int.strip_host_suffix,
            headers_config: config.done(),
        })
    }
}

impl<'a> Deserialize<'a> for SingleFile {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        pub struct Internal {
            pub path: PathBuf,
            pub content_type: Option<String>,
            pub pool: DiskPoolName,
            pub extra_headers: HashMap<String, String>,
        }
        let int = Internal::deserialize(d)?;
        if header_contains(&int.extra_headers, "Content-Type") {
            return Err(D::Error::custom("Content-Type must be specified as \
                `content-type` parameter rather than in `extra-headers` \
                in `!SingleFile` handler."));
        }
        let mut config = HeadersConfig::new();
        config.no_text_charset(); // TODO(tailhook) backward compatibility
        if int.content_type.is_some() {
            config.content_type(false);
        }
        return Ok(SingleFile {
            path: int.path,
            content_type: int.content_type,
            pool: int.pool,
            extra_headers: int.extra_headers,
            headers_config: config.done(),
        })
    }
}

impl<'a> Deserialize<'a> for VersionedStatic {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
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
        let int = Internal::deserialize(d)?;
        let mut config = HeadersConfig::new();
        match int.text_charset {
            Some(ref charset) => { config.text_charset(charset); }
            None => { config.no_text_charset(); }
        }
        if header_contains(&int.extra_headers, "Content-Type") {
            config.content_type(false);
        }
        let config = config.done();
        return Ok(VersionedStatic {
            version_len: int.version_split.iter().map(|&x| x as usize).sum(),
            fallback: Arc::new(Static {
                mode: int.fallback_mode.clone(),
                path: int.plain_root.clone(),
                text_charset: int.text_charset.clone(),
                pool: int.pool.clone(),
                extra_headers: int.extra_headers.clone(),
                index_files: Vec::new(),
                generate_index: false,
                generated_index_max_files: 0,
                strip_host_suffix: None,
                headers_config: config.clone(),
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
            headers_config: config,
        })
    }
}

pub fn header_contains(map: &HashMap<String, String>, name: &str) -> bool {
    map.iter().any(|(header, _)| header.eq_ignore_ascii_case(name))
}

impl PartialEq for Static {
    fn eq(&self, other: &Static) -> bool {
        let Static {
            mode: ref a_mode,
            path: ref a_path,
            text_charset: ref a_text_charset,
            pool: ref a_pool,
            extra_headers: ref a_extra_headers,
            strip_host_suffix: ref a_strip_host_suffix,
            index_files: ref a_index_files,
            generate_index: ref a_generate_index,
            generated_index_max_files: ref a_generated_index_max_files,
            headers_config: _,
        } = *self;
        let Static {
            mode: ref b_mode,
            path: ref b_path,
            text_charset: ref b_text_charset,
            pool: ref b_pool,
            extra_headers: ref b_extra_headers,
            strip_host_suffix: ref b_strip_host_suffix,
            index_files: ref b_index_files,
            generate_index: ref b_generate_index,
            generated_index_max_files: ref b_generated_index_max_files,
            headers_config: _,
        } = *other;
        return a_mode == b_mode &&
               a_path == b_path &&
               a_text_charset == b_text_charset &&
               a_pool == b_pool &&
               a_extra_headers == b_extra_headers &&
               a_strip_host_suffix == b_strip_host_suffix &&
               a_index_files == b_index_files &&
               a_generate_index == b_generate_index &&
               a_generated_index_max_files == b_generated_index_max_files;

    }
}

impl PartialEq for SingleFile {
    fn eq(&self, other: &SingleFile) -> bool {
        let SingleFile {
            path: ref a_path,
            content_type: ref a_content_type,
            pool: ref a_pool,
            extra_headers: ref a_extra_headers,
            headers_config: _,
        } = *self;
        let SingleFile {
            path: ref b_path,
            content_type: ref b_content_type,
            pool: ref b_pool,
            extra_headers: ref b_extra_headers,
            headers_config: _,
        } = *other;
        return a_path == b_path &&
               a_content_type == b_content_type &&
               a_pool == b_pool &&
               a_extra_headers == b_extra_headers;
    }
}

impl PartialEq for VersionedStatic {
    fn eq(&self, other: &VersionedStatic) -> bool {
        let VersionedStatic {
            versioned_root: ref a_versioned_root,
            plain_root: ref a_plain_root,
            version_arg: ref a_version_arg,
            version_split: ref a_version_split,
            version_chars: ref a_version_chars,
            fallback_to_plain: ref a_fallback_to_plain,
            fallback_mode: ref a_fallback_mode,
            text_charset: ref a_text_charset,
            pool: ref a_pool,
            extra_headers: ref a_extra_headers,
            version_len: _,
            fallback: _,
            headers_config: _,
        } = *self;
        let VersionedStatic {
            versioned_root: ref b_versioned_root,
            plain_root: ref b_plain_root,
            version_arg: ref b_version_arg,
            version_split: ref b_version_split,
            version_chars: ref b_version_chars,
            fallback_to_plain: ref b_fallback_to_plain,
            fallback_mode: ref b_fallback_mode,
            text_charset: ref b_text_charset,
            pool: ref b_pool,
            extra_headers: ref b_extra_headers,
            version_len: _,
            fallback: _,
            headers_config: _,
        } = *other;
        return a_versioned_root == b_versioned_root &&
               a_plain_root == b_plain_root &&
               a_version_arg == b_version_arg &&
               a_version_split == b_version_split &&
               a_version_chars == b_version_chars &&
               a_fallback_to_plain == b_fallback_to_plain &&
               a_fallback_mode == b_fallback_mode &&
               a_text_charset == b_text_charset &&
               a_pool == b_pool &&
               a_extra_headers == b_extra_headers;
    }
}

impl Eq for Static {}
impl Eq for SingleFile {}
impl Eq for VersionedStatic {}
