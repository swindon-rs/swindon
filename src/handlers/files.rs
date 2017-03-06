use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf, Component};
use std::sync::{Arc, RwLock};

use futures_cpupool::CpuPool;
use futures::{Future};
use futures::future::{ok};
use mime_guess::guess_mime_type;
use mime::TopLevel;
use tk_http::server::Error;
use tk_http::Status;
use tk_sendfile::DiskPool;

use config;
use config::static_files::{Static, Mode, SingleFile};
use default_error_page::{serve_error_page, error_page};
use incoming::{Input, Request, Reply, Transport};
use incoming::reply;
use intern::{DiskPoolName};


quick_error! {
    #[derive(Debug)]
    enum FileError {
        Sendfile(err: io::Error) {
            description("sendfile error")
            cause(err)
        }
    }
}


lazy_static! {
    static ref POOLS: RwLock<HashMap<DiskPoolName, (u64, DiskPool)>> =
        RwLock::new(HashMap::new());
    static ref DEFAULT: DiskPoolName = DiskPoolName::from("default");
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
    let mime = guess_mime_type(&path);
    inp.debug.set_fs_path(&path);
    let pool = get_pool(&settings.pool);
    let settings = settings.clone();
    reply(inp, move |mut e| {
        Box::new(pool.open(path)
            .then(move |res| match res {
                Ok(file) => {
                    e.status(Status::Ok);
                    e.add_length(file.size());
                    if !settings.overrides_content_type {
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
    let path = Path::new(path.trim_left_matches('/'));
    let mut buf = settings.path.to_path_buf();
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
                buf.push(name);
            }
            None => return Err(()),
        }
    }
    for cmp in path.components() {
        match cmp {
            Component::Normal(chunk) => {
                buf.push(chunk);
            }
            _ => return Err(()),
        }
    }
    Ok(buf)
}

pub fn serve_file<S: Transport>(settings: &Arc<SingleFile>, mut inp: Input)
    -> Request<S>
{
    if !inp.headers.path().is_some() {
        // Star or authority
        return serve_error_page(Status::Forbidden, inp);
    };
    inp.debug.set_fs_path(&settings.path);
    let pool = get_pool(&settings.pool);
    let settings = settings.clone();
    reply(inp, move |mut e| {
        Box::new(pool.open(settings.path.clone())
            .then(move |res| match res {
                Ok(file) => {
                    e.status(Status::Ok);
                    e.add_length(file.size());
                    e.add_header("Content-Type", &settings.content_type);
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
                // TODO(tailhook) find out if we want to expose other
                // errors, for example "Permission denied" and "is a directory"
                Err(_) => {
                    // TODO: log error.
                    Box::new(error_page(Status::InternalServerError, e))
                }
            }))
    })
}

fn new_pool(cfg: &config::Disk) -> DiskPool {
    DiskPool::new(CpuPool::new(cfg.num_threads))
}

fn get_pool(name: &DiskPoolName) -> DiskPool {
    let pools = POOLS.read().expect("readlock for pools");
    match pools.get(name) {
        Some(&(_, ref x)) => x.clone(),
        None => {
            warn!("Unknown disk pool {}, using default", name);
            pools.get(&*DEFAULT).unwrap().1.clone()
        }
    }
}

pub fn update_pools(config: &HashMap<DiskPoolName, config::Disk>) {
    let mut pools = POOLS.write().expect("writelock for pools");
    for (name, props) in config {
        let mut hasher = DefaultHasher::new();
        props.hash(&mut hasher);
        let new_hash = hasher.finish();
        match pools.entry(name.clone()) {
            Occupied(mut o) => {
                let (ref mut old_hash, ref mut old_pool) = *o.get_mut();
                debug!("Upgrading disk pool {} to {:?}", name, props);
                if *old_hash != new_hash {
                    *old_pool = new_pool(props);
                    *old_hash = new_hash;
                }
            }
            Vacant(v) => {
                v.insert((new_hash, new_pool(props)));
            }
        }
    }
    if !pools.contains_key(&*DEFAULT) {
        let cfg = config::Disk {
            num_threads: 40,
        };
        let mut hasher = DefaultHasher::new();
        cfg.hash(&mut hasher);
        let hash = hasher.finish();
        pools.insert(DEFAULT.clone(), (hash, new_pool(&cfg)));
    }
}
