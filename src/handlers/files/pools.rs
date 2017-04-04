use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::fs::{File, metadata};
use std::ffi::OsStr;
use std::hash::{Hash, Hasher};
use std::io;
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
use config::static_files::{Static, Mode, SingleFile, VersionedStatic};
use default_error_page::{serve_error_page, error_page};
use incoming::{Input, Request, Reply, Transport};
use incoming::reply;
use intern::{DiskPoolName};
use runtime::Runtime;


#[derive(Clone)]
pub struct DiskPools(Arc<RwLock<PoolsInternal>>);

struct PoolsInternal {
    pools: HashMap<DiskPoolName, (u64, DiskPool)>,
    default: DiskPool,
    meter: Meter,
}


fn new_pool(name: &DiskPoolName, cfg: &config::Disk, meter: &Meter)
    -> DiskPool
{
    let m1 = meter.clone();
    let m2 = meter.clone();
    DiskPool::new(futures_cpupool::Builder::new()
        .pool_size(cfg.num_threads)
        .name_prefix(format!("disk-{}-", name))
        .after_start(move || m1.track_current_thread_by_name())
        .before_stop(move || m2.untrack_current_thread())
        .create())
}

pub fn get_pool(runtime: &Runtime, name: &DiskPoolName) -> DiskPool {
    let pools = runtime.disk_pools.0.read().expect("readlock for pools");
    match pools.pools.get(name) {
        Some(&(_, ref x)) => x.clone(),
        None => {
            warn!("Unknown disk pool {}, using default", name);
            pools.default.clone()
        }
    }
}

impl DiskPools {
    pub fn new(meter: &Meter) -> DiskPools {
        let mut pools = HashMap::new();
        let cfg = config::Disk {
            num_threads: 40,
        };
        let mut hasher = DefaultHasher::new();
        cfg.hash(&mut hasher);
        let hash = hasher.finish();
        let dname = DiskPoolName::from("default");
        let default = new_pool(&dname, &cfg, meter);
        pools.insert(dname, (hash, default.clone()));

        DiskPools(Arc::new(RwLock::new(PoolsInternal {
            pools: pools,
            default: default,
            meter: meter.clone(),
        })))
    }
    pub fn update(&self, config: &HashMap<DiskPoolName, config::Disk>) {
        let mut pools = &mut *self.0.write().expect("writelock for pools");
        for (name, props) in config {
            let mut hasher = DefaultHasher::new();
            props.hash(&mut hasher);
            let new_hash = hasher.finish();
            match pools.pools.entry(name.clone()) {
                Occupied(mut o) => {
                    let (ref mut old_hash, ref mut old_pool) = *o.get_mut();
                    debug!("Upgrading disk pool {} to {:?}", name, props);
                    if *old_hash != new_hash {
                        *old_pool = new_pool(name, props, &pools.meter);
                        *old_hash = new_hash;
                    }
                }
                Vacant(v) => {
                    v.insert((new_hash, new_pool(name, props, &pools.meter)));
                }
            }
        }
        pools.default = pools.pools[&DiskPoolName::from("default")].1.clone();
    }
}
