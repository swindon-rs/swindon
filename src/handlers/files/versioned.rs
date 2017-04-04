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


pub fn serve_versioned<S: Transport>(settings: &Arc<VersionedStatic>,
    mut inp: Input)
    -> Request<S>
{
    unimplemented!();
}
