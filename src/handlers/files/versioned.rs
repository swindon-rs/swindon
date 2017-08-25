use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc};
use std::str::from_utf8;

use tk_http::Status;
use http_file_headers::{Input as HeadersInput, Output};

use config::static_files::{VersionChars, VersionedStatic};
use default_error_page::{error_page};
use incoming::{Input, Request, Transport, reply};
use handlers::files::decode::decode_component;
use handlers::files::normal;
use handlers::files::pools::get_pool;
use handlers::files::common::reply_file;

quick_error! {
    #[derive(Debug, Copy, Clone)]
    pub enum VersionError {
        NoVersion
        BadVersion  // length or chars
        InvalidPath
        NoFile
    }
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
        use config::static_files::FallbackMode::*;

        let res = path.as_ref()
            .map_err(|e| *e)
            .map(|path| hinp.probe_file(&path))
            .and_then(|x| match x {
                Ok(Output::NotFound) => Err(NoFile),
                x => Ok(x),
            });
        let res = match (res, &npath, settings.fallback_to_plain) {
            (Ok(x), _, _) => x,
            (Err(_), &Some(ref pp), always)
            | (Err(NoFile), &Some(ref pp), no_file)
            | (Err(BadVersion), &Some(ref pp), no_file)
            | (Err(InvalidPath), &Some(ref pp), no_file)
            | (Err(NoVersion), &Some(ref pp), no_file)
            | (Err(BadVersion), &Some(ref pp), bad_version)
            | (Err(NoVersion), &Some(ref pp), bad_version)
            | (Err(NoVersion), &Some(ref pp), no_version)
            => {
                hinp.probe_file(pp)
            }
            (Err(_), _, _) => {
                return Ok(Output::NotFound);
            }
        };
        return res.map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                Status::Forbidden
            } else {
                error!("Error reading file {:?} / {:?}: {}", path, npath, e);
                Status::InternalServerError
            }
        });
    });

    reply_file(inp, pool, fut, move |e| {
        e.add_extra_headers(&settings2.extra_headers);
    }, |e| {
        // TODO(tailhook) autoindex
        error_page(Status::Forbidden, e)
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
