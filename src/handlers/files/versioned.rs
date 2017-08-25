use std::fs::{File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc};
use std::str::from_utf8;

use futures::{Future};
use futures::future::{ok};
use mime_guess::guess_mime_type;
use mime::{TopLevel, Mime};
use tk_http::server::Error;
use tk_http::Status;

use config::static_files::{VersionChars, VersionedStatic};
use default_error_page::{error_page};
use incoming::{Input, Request, Reply, Transport};
use incoming::reply;
use handlers::files::decode::decode_component;
use handlers::files::normal;
use handlers::files::pools::get_pool;

quick_error! {
    #[derive(Debug, Copy, Clone)]
    pub enum VersionError {
        NoVersion
        BadVersion  // length or chars
        InvalidPath
        NoFile
    }
}


#[cfg(unix)]
struct PathOpen {
    version_path: Result<PathBuf, VersionError>,
    plain_path: Option<PathBuf>,
    settings: Arc<VersionedStatic>,
    file: Option<(File, u64, Mime)>,
}

#[cfg(windows)]
struct PathOpen {
    version_path: Option<PathBuf>,
    plain_path: Option<PathBuf>,
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
    let path = path(settings, &inp);
    let npath = normal::path(&settings.fallback, &inp).ok();
    inp.debug.set_fs_path( // TODO(tailhook)
        &path.as_ref().ok().map(|x| -> &Path { x.as_ref() })
        .or(npath.as_ref().map(|x| -> &Path { x.as_ref() }))
        .unwrap_or(&Path::new("")));
    let pool = get_pool(&inp.runtime, &settings.pool);
    let settings = settings.clone();
    if path.is_err() && npath.is_none() {
        inp.debug.set_deny(path.unwrap_err().to_header_string());
        return reply(inp, move |e| {
            Box::new(error_page(Status::NotFound, e))
        });
    }
    unimplemented!();
    /*
    reply(inp, move |mut e| {
        Box::new(pool.open(PathOpen::new(path, npath, &settings))
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
    */
}

impl PathOpen {
    fn new(vpath: Result<PathBuf, VersionError>, npath: Option<PathBuf>,
        settings: &Arc<VersionedStatic>)
        -> PathOpen
    {
        PathOpen {
            version_path: vpath,
            plain_path: npath,
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

/*
impl FileOpener for PathOpen {
    fn open(&mut self) -> Result<(&FileReader, u64), io::Error> {
        use self::VersionError::*;
        use config::static_files::FallbackMode::*;
        if self.file.is_none() {
            let vers = match self.version_path.as_ref().map(|x| File::open(&x))
            {
                Ok(Ok(file)) => Ok(file),
                Ok(Err(ref e)) if e.kind() == io::ErrorKind::NotFound => {
                    Err(NoFile)
                }
                Ok(Err(e)) => {
                    debug!("Error opening version: {}", e);
                    return Err(e);
                }
                Err(&e) => Err(e),
            };
            let file = match (vers, &self.plain_path,
                              self.settings.fallback_to_plain)
            {
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
                    File::open(pp)?
                }
                (Err(_), _, _) => {
                    return Err(io::ErrorKind::NotFound.into());
                }
            };
            let meta = file.metadata()?;
            if meta.is_dir() {
                return Err(io::ErrorKind::Other.into());
            } else {
                let mime = guess_mime_type(
                    &self.version_path.as_ref().ok()
                    .unwrap_or(self.plain_path.as_ref().unwrap()));
                self.file = Some((wrap_file(file), meta.len(), mime));
            }
        }
        Ok(self.file.as_ref()
            .map(|&(ref f, s, _)| (f as &FileReader, s)).unwrap())
    }
}
*/


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
