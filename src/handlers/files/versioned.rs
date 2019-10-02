use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc};
use std::str::from_utf8;
use std::time::{SystemTime, Duration};

use http_file_headers::{Input as HeadersInput, Output};
use httpdate::HttpDate;
use tk_http::Status;

use crate::config::static_files::{VersionChars, VersionedStatic};
use crate::default_error_page::{error_page};
use crate::incoming::{Input, Request, Transport, reply};
use crate::handlers::files::decode::decode_component;
use crate::handlers::files::normal;
use crate::handlers::files::pools::get_pool;
use crate::handlers::files::common::{reply_file, NotFile};


const VERSIONED_CACHE: &str = "public, max-age=31536000, immutable";
const VERSIONED_EXPIRES: u64 = 365*86400;
const UNVERSIONED_CACHE: &str = "no-cache, no-store, must-revalidate";

quick_error! {
    #[derive(Debug, Copy, Clone)]
    pub enum VersionError {
        NoVersion
        BadVersion  // length or chars
        InvalidPath
        NoFile
    }
}

pub enum Cache {
    NoHeader,
    NoCache,
    GoodCache,
}

fn find_param<'x>(query: &'x str, arg: &str) -> Option<&'x str> {
    for pair in query.split('&') {
        if pair.starts_with(arg) && pair.len() > arg.len()
            && pair[arg.len()..].starts_with("=")
        {
            return Some(&pair[arg.len()+1..]);
        }
    }
    return None;
}

fn path(settings: &VersionedStatic, inp: &Input)
    -> Result<PathBuf, VersionError>
{
    let path = inp.headers.path().unwrap_or("/");
    let path = match path.find(|c| c == '#') {
        Some(idx) => &path[..idx],
        None => path
    };
    let query = path.find(|c| c == '?').ok_or(VersionError::NoVersion)?;
    let (path, query) = path.split_at(query);
    let version = find_param(&query[1..], &settings.version_arg)
        .ok_or(VersionError::NoVersion)?;

    if version.len() != settings.version_len {
        return Err(VersionError::BadVersion);
    }
    match settings.version_chars {
        VersionChars::lowercase_hex => {
            for &c in version.as_bytes() {
                if c < b'0' || (c > b'9' && c < b'a') || c > b'z' {
                    return Err(VersionError::BadVersion);
                }
            }
        }
    }

    let file_name = match path.rfind('/') {
        Some(idx) => &path[idx+1..],
        None => path,
    };
    let mut buf = Vec::with_capacity(
        settings.version_len*2 + 1 + file_name.len());
    let mut offset = 0;
    for &chunk in &settings.version_split {
        buf.extend(version[offset..offset+chunk as usize].as_bytes());
        buf.push(b'/');
        offset += chunk as usize;
    }
    buf.pop();
    buf.push(b'-');
    decode_component(&mut buf, file_name)
        .map_err(|_| VersionError::InvalidPath)?;

    // only valid utf-8 supported so far
    let utf8 = from_utf8(&buf).map_err(|_| VersionError::InvalidPath)?;
    Ok(settings.versioned_root.join(utf8))
}

pub fn serve_versioned<S: Transport>(settings: &Arc<VersionedStatic>,
    mut inp: Input)
    -> Request<S>
{
    let path = path(settings, &inp);
    let npath = normal::path(&settings.fallback, &inp).ok();
    inp.debug.set_fs_path( // TODO(tailhook)
        &path.as_ref().ok().map(|x| -> &Path { x.as_ref() })
        .or(npath.as_ref().map(|x| -> &Path { x.as_ref() }))
        .unwrap_or(&Path::new("")));
    let pool = get_pool(&inp.runtime, &settings.pool);
    let settings = settings.clone();
    let settings2 = settings.clone();
    if path.is_err() && npath.is_none() {
        inp.debug.set_deny(path.unwrap_err().to_header_string());
        return reply(inp, move |e| {
            Box::new(error_page(Status::NotFound, e))
        });
    }

    let hinp = HeadersInput::from_headers(&settings.headers_config,
        inp.headers.method(), inp.headers.headers());
    let fut = pool.spawn_fn(move || {
        use self::VersionError::*;
        use crate::config::static_files::FallbackMode::*;

        let res = path.as_ref()
            .map_err(|e| *e)
            .map(|path| hinp.probe_file(&path))
            .and_then(|x| match x {
                Ok(Output::NotFound) => Err(NoFile),
                x => Ok(x),
            });
        let res = match (res, &npath, settings.fallback_to_plain) {
            (Ok(x), _, _) => x.map(|f| (f, Cache::GoodCache)),
            (Err(e@_), &Some(ref pp), always)
            | (Err(e@NoFile), &Some(ref pp), no_file)
            | (Err(e@BadVersion), &Some(ref pp), no_file)
            | (Err(e@InvalidPath), &Some(ref pp), no_file)
            | (Err(e@NoVersion), &Some(ref pp), no_file)
            | (Err(e@BadVersion), &Some(ref pp), bad_version)
            | (Err(e@NoVersion), &Some(ref pp), bad_version)
            | (Err(e@NoVersion), &Some(ref pp), no_version)
            => {
                // TODO(tailhook) update debug path
                hinp.probe_file(pp).map(|file| {
                    let cache = match e {
                        NoVersion => Cache::NoHeader,
                        BadVersion => Cache::NoHeader,
                        InvalidPath => Cache::NoHeader,
                        NoFile => Cache::NoCache,
                    };
                    (file, cache)
                })
            }
            (Err(_), _, _) => {
                Ok((Output::NotFound, Cache::NoHeader))
            }
        };
        return res.map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                (NotFile::Status(Status::Forbidden), Cache::NoHeader)
            } else {
                error!("Error reading file {:?} / {:?}: {}", path, npath, e);
                (NotFile::Status(Status::InternalServerError),
                 Cache::NoHeader)
            }
        });
    });

    reply_file(inp, pool, fut, move |e, cache| {
        match cache {
            Cache::NoHeader => {}
            Cache::NoCache => {
                e.add_header("Cache-Control", UNVERSIONED_CACHE.as_bytes());
                e.add_header("Expires", b"0");
            }
            Cache::GoodCache => {
                e.add_header("Cache-Control", VERSIONED_CACHE.as_bytes());
                let expires = SystemTime::now() +
                    Duration::new(VERSIONED_EXPIRES, 0);
                e.format_header("Expires", &HttpDate::from(expires));
            }
        }
        e.add_extra_headers(&settings2.extra_headers);
    })
}

impl VersionError {
    fn to_header_string(&self) -> &'static str {
        match *self {
            VersionError::NoVersion => "no-version",
            VersionError::BadVersion => "bad-version",
            VersionError::InvalidPath => "invalid-path",
            VersionError::NoFile => "no-file",
        }
    }
}
