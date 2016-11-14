use std::sync::{Arc, RwLock};
use std::path::{Path, PathBuf, Component};
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::os::unix::io::AsRawFd;
use std::hash::{Hash, Hasher, SipHasher};

use futures::{BoxFuture, Future};
use either::Either;
use minihttp::{Error, Status};
use minihttp::request::Request;
use mime::TopLevel;
use mime_guess::guess_mime_type;
use tk_sendfile::DiskPool;
use tk_bufstream::IoBuf;
use tokio_core::io::Io;
use futures_cpupool::CpuPool;

use intern::{DiskPoolName};
use config;
use config::static_files::{Static, Mode, SingleFile};
use {Pickler};


lazy_static! {
    static ref POOLS: RwLock<HashMap<DiskPoolName, (u64, DiskPool)>> =
        RwLock::new(HashMap::new());
    static ref DEFAULT: DiskPoolName = DiskPoolName::from("default");
}


pub fn serve<S>(mut response: Pickler<S>, path: PathBuf, settings: Arc<Static>)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + AsRawFd + 'static,
{
    // TODO(tailhook) check for symlink attacks
    let mime = guess_mime_type(&path);
    let debug_path = if response.debug_routing() {
        Some(path.clone())
    } else {
        None
    };
    get_pool(&settings.pool).open(path)
    .map_err(Into::into)
    .and_then(move |file| {
        response.status(Status::Ok);
        response.add_length(file.size());
        match (&mime.0, &settings.text_charset) {
            (&TopLevel::Text, &Some(ref enc)) => {
                response.format_header("Content-Type", format_args!(
                    "{}/{}; charset={}", mime.0, mime.1, enc));
            }
            _ => {
                response.format_header("Content-Type", mime);
            }
        }
        if response.debug_routing() {  // just to be explicit
            if let Some(path) = debug_path {
                response.format_header("X-Swindon-File-Path",
                    format_args!("{:?}", path));
            }
        }
        response.add_extra_headers(&settings.extra_headers);
        if response.done_headers() {
            Either::A(response.steal_socket()
                .and_then(|sock| file.write_into(sock))
                .map_err(Into::into))
        } else {
            Either::B(response.done())
        }
    }).boxed()
}

pub fn path(settings: &Static, suffix: &str, req: &Request)
    -> Result<PathBuf, ()>
{
    let path = match settings.mode {
        Mode::relative_to_domain_root => &req.path,
        Mode::relative_to_route => suffix,
    };
    let path = Path::new(path.trim_left_matches('/'));
    let mut buf = settings.path.to_path_buf();
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

pub fn serve_file<S>(mut response: Pickler<S>, settings: Arc<SingleFile>)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + AsRawFd + 'static,
{
    get_pool(&settings.pool).open(settings.path.clone())
    // TODO(tailhook) this is not very good error
    .map_err(Into::into)
    .and_then(move |file| {
        response.status(Status::Ok);
        response.add_length(file.size());
        response.add_header("Content-Type", &settings.content_type);
        if response.debug_routing() {
            response.format_header("X-Swindon-File-Path",
                format_args!("{:?}", settings.path));
        }
        response.add_extra_headers(&settings.extra_headers);
        if response.done_headers() {
            Either::A(response.steal_socket()
                .and_then(|sock| file.write_into(sock))
                .map_err(Into::into))
        } else {
            Either::B(response.done())
        }
    }).boxed()
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
        let mut hasher = SipHasher::new();
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
        let mut hasher = SipHasher::new();
        cfg.hash(&mut hasher);
        let hash = hasher.finish();
        pools.insert(DEFAULT.clone(), (hash, new_pool(&cfg)));
    }
}
