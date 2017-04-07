use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::fs::{File, metadata};
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, Mutex};
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
use config::static_files::{Static, Mode, SingleFile, VersionedStatic};
use default_error_page::{serve_error_page, error_page};
use incoming::{Input, Request, Reply, Transport};
use incoming::reply;
use intern::{DiskPoolName};
use runtime::Runtime;
use handlers::files::FileError;
use handlers::files::decode::decode_component;
use handlers::files::pools::get_pool;


#[cfg(unix)]
struct PathOpen {
    path: PathBuf,
    settings: Arc<Static>,
    file: Option<(File, u64, Mime)>,
}

#[cfg(windows)]
struct PathOpen {
    path: PathBuf,
    settings: Arc<Static>,
    file: Option<(Mutex<File>, u64, Mime)>,
}

pub fn serve_dir<S: Transport>(settings: &Arc<Static>, mut inp: Input)
    -> Request<S>
{
    // TODO(tailhook) check for symlink attacks
    let path = match path(settings, &inp) {
        Ok(p) => p,
        Err(()) => {
            return serve_error_page(Status::Forbidden, inp);
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

fn path(settings: &Static, inp: &Input) -> Result<PathBuf, ()> {
    let path = match settings.mode {
        Mode::relative_to_domain_root | Mode::with_hostname => {
            inp.headers.path().unwrap_or("/")
        }
        Mode::relative_to_route => inp.suffix,
    };
    let path = match path.find(|c| c == '?' || c == '#') {
        Some(idx) => &path[..idx],
        None => path
    };
    let mut buf = Vec::with_capacity(path.len());
    if settings.mode == Mode::with_hostname {
        match inp.headers.host()  {
            Some(host) => {
                if host.find("/").is_some() {
                    // no slashes allowed
                    return Err(());
                }
                let name: &str = if let Some(colon) = host.find(":") {
                    &host[..colon]
                } else {
                    &host[..]
                };
                let name = if let Some(ref suf) = settings.strip_host_suffix {
                    if suf.len() >= name.len() {
                        // empty prefix is not allowed yet
                        return Err(());
                    }
                    if !name.ends_with(suf) {
                        // only this suffix should work
                        return Err(());
                    }
                    let final_dot = name.len() - suf.len() - 1;
                    if !name[final_dot..].starts_with('.') {
                        return Err(())
                    }
                    &name[..final_dot]
                } else {
                    name
                };
                buf.extend(name.as_bytes());
            }
            None => return Err(()),
        }
    }
    for cmp in path.split("/") {
        match cmp {
            "" | "." | "%2e" | "%2E" => {},
            ".." | "%2e." | "%2E." | ".%2e" | ".%2E"
            | "%2e%2e" | "%2E%2e" | "%2e%2E" | "%2E%2E" => return Err(()),
            _ => {
                if buf.len() > 0 {
                    buf.push(b'/');
                }
                decode_component(&mut buf, cmp)?;
            }
        }
    }

    // assert that we're not serving from root, this is a security check
    assert!(buf.len() == 0 || buf[0] != b'/');

    // only valid utf-8 supported so far
    let utf8 = from_utf8(&buf).map_err(|_| ())?;
    Ok(settings.path.join(utf8))
}


impl PathOpen {
    fn new(path: PathBuf, settings: &Arc<Static>) -> PathOpen {
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

impl IntoFileOpener for PathOpen {
    type Opener = PathOpen;
    fn into_file_opener(self) -> Self::Opener {
        self
    }
}

fn find_index(path: &Path, settings: &Arc<Static>)
    -> Result<(File, u64, Mime), io::Error>
{
    for file_name in &settings.index_files {
        let file = match File::open(path.join(file_name)) {
            Ok(x) => x,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                continue;
            }
            Err(e) => return Err(e),
        };
        let meta = file.metadata()?;
        if meta.is_file() {
            let mime = guess_mime_type(&file_name);
            return Ok((file, meta.len(), mime));
        }
    }
    return Err(io::ErrorKind::Other.into());
}

#[cfg(unix)]
fn wrap_file(file: File) -> File {
    file
}

#[cfg(windows)]
fn wrap_file(file: File) -> Mutex<File> {
    Mutex::new(file)
}

impl FileOpener for PathOpen {
    fn open(&mut self) -> Result<(&FileReader, u64), io::Error> {
        if self.file.is_none() {
            let file = File::open(&self.path)?;
            let meta = file.metadata()?;
            if meta.is_dir() {
                if self.settings.index_files.len() > 0 &&
                    metadata(&self.path)?.is_dir()
                {
                    let (f, mt, mm) = find_index(&self.path, &self.settings)?;
                    self.file = Some((wrap_file(f), mt, mm));
                } else {
                    return Err(io::ErrorKind::Other.into());
                }
            } else {
                let mime = guess_mime_type(&self.path);
                self.file = Some((wrap_file(file), meta.len(), mime));
            }
        }
        Ok(self.file.as_ref()
            .map(|&(ref f, s, _)| (f as &FileReader, s)).unwrap())
    }
}

