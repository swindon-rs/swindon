use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::fs::{File, metadata};
use std::ffi::OsStr;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf, Component};
use std::sync::{Arc, RwLock};
use std::str::from_utf8;

use futures_cpupool;
use futures::{Future};
use futures::future::{ok};
use mime_guess::guess_mime_type;
use mime::{TopLevel, Mime};
use tk_http::server::Error;
use tk_http::Status;
use tk_sendfile::{DiskPool, FileOpener, IntoFileOpener, FileReader};
use self_meter_http::Meter;

use config;
use config::static_files::{VersionChars, FallbackMode, Mode, VersionedStatic};
use default_error_page::{serve_error_page, error_page};
use incoming::{Input, Request, Reply, Transport};
use incoming::reply;
use intern::{DiskPoolName};
use runtime::Runtime;
use handlers::files::FileError;
use handlers::files::decode::decode_component;
use handlers::files::pools::get_pool;


quick_error! {
    #[derive(Debug)]
    pub enum VersionError {
        NoVersion
        BadVersion  // length or chars
        InvalidPath
    }
}


#[cfg(unix)]
struct PathOpen {
    path: PathBuf,
    settings: Arc<VersionedStatic>,
    file: Option<(File, u64, Mime)>,
}

#[cfg(windows)]
struct PathOpen {
    path: PathBuf,
    settings: Arc<VersionedStatic>,
    file: Option<(Mutex<File>, u64, Mime)>,
}

#[cfg(unix)]
fn wrap_file(file: File) -> File {
    file
}

#[cfg(windows)]
fn wrap_file(file: File) -> Mutex<File> {
    Mutex::new(file)
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
    let path = match path(settings, &inp) {
        Ok(p) => p,
        Err(e) => {
            inp.debug.set_deny(e.to_header_string());
            return serve_error_page(Status::NotFound, inp);
        }
    };
    inp.debug.set_fs_path(&path);
    let pool = get_pool(&inp.runtime, &settings.pool);
    let settings = settings.clone();
    reply(inp, move |mut e| {
        Box::new(pool.open(PathOpen::new(path, &settings))
            .then(move |res| match res {
                Ok(file) => {
                    e.status(Status::Ok);
                    e.add_length(file.size());
                    if !settings.overrides_content_type {
                        let mime = file.get_inner().get_mime();
                        match (&mime.0, &settings.text_charset) {
                            (&TopLevel::Text, &Some(ref enc)) => {
                                e.format_header("Content-Type", format_args!(
                                    "{}/{}; charset={}", mime.0, mime.1, enc));
                            }
                            _ => {
                                e.format_header("Content-Type", mime);
                            }
                        }
                    }
                    e.add_extra_headers(&settings.extra_headers);
                    if e.done_headers() {
                        Box::new(e.raw_body()
                            .and_then(|raw_body| file.write_into(raw_body))
                            .map(|raw_body| raw_body.done())
                            .map_err(FileError::Sendfile)
                            .map_err(Error::custom))
                        as Reply<_>
                    } else {
                        Box::new(ok(e.done()))
                    }
                }
                Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                    Box::new(error_page(Status::NotFound, e))
                }
                // One of the known `Other` issues is when path refers to
                // a directory rather than regular file
                Err(ref err) if err.kind() == io::ErrorKind::Other => {
                    Box::new(error_page(Status::Forbidden, e))
                }
                // TODO(tailhook) find out if we want to expose other
                // errors, for example "Permission denied" and "is a directory"
                Err(_) => {
                    Box::new(error_page(Status::InternalServerError, e))
                }
            }))
    })
}

impl PathOpen {
    fn new(path: PathBuf, settings: &Arc<VersionedStatic>) -> PathOpen {
        PathOpen {
            path: path,
            settings: settings.clone(),
            file: None,
        }
    }
    fn get_mime(&self) -> &Mime {
        self.file.as_ref()
            .map(|&(_, _, ref m)| m)
            .unwrap()
    }
}

impl FileOpener for PathOpen {
    fn open(&mut self) -> Result<(&FileReader, u64), io::Error> {
        if self.file.is_none() {
            let file = File::open(&self.path)?;
            let meta = file.metadata()?;
            if meta.is_dir() {
                return Err(io::ErrorKind::Other.into());
            } else {
                let mime = guess_mime_type(&self.path);
                self.file = Some((wrap_file(file), meta.len(), mime));
            }
        }
        Ok(self.file.as_ref()
            .map(|&(ref f, s, _)| (f as &FileReader, s)).unwrap())
    }
}

impl IntoFileOpener for PathOpen {
    type Opener = PathOpen;
    fn into_file_opener(self) -> Self::Opener {
        self
    }
}

impl VersionError {
    fn to_header_string(&self) -> &'static str {
        match *self {
            VersionError::NoVersion => "no-version",
            VersionError::BadVersion => "bad-version",
            VersionError::InvalidPath => "invalid-path",
        }
    }
}
