use std::fmt::{Display, Write};
use std::sync::Arc;
use std::path::{Path, PathBuf};

use tk_http::server::Head;

use intern::{HandlerName, Authorizer};
use config::Config;
use routing::Route;
use request_id::RequestId;

pub struct Debug(Option<Box<DebugInfo>>);

struct DebugInfo {
    route: Option<Route>,
    fs_path: Option<PathBuf>,
    config: Arc<Config>,
    request_id: RequestId,
    allow: String,
    deny: String,
}

impl Debug {
    pub fn new(_head: &Head, request_id: RequestId, cfg: &Arc<Config>)
        -> Debug
    {
        if cfg.debug_routing {
            Debug(Some(Box::new(DebugInfo {
                route: None,
                fs_path: None,
                config: cfg.clone(),
                request_id: request_id,
                allow: String::new(),
                deny: String::new(),
            })))
        } else {
            Debug(None)
        }
    }
    /// Add route information
    ///
    /// # Panics
    ///
    /// Panics if route is already set (only in debug mode)
    pub fn set_route(&mut self, route: &Route) {
        if let Some(ref mut dinfo) = self.0 {
            debug_assert!(dinfo.route.is_none());
            dinfo.route = Some(route.clone());
        }
    }

    pub fn get_route(&self) -> Option<&str> {
        self.0.as_ref().map(|dinfo| {
            dinfo.route.as_ref().map(|x| &x.handler_name[..])
            .unwrap_or("-- no route --")
        })
    }

    pub fn set_fs_path<P: AsRef<Path>>(&mut self, path: P) {
        if let Some(ref mut dinfo) = self.0 {
            dinfo.fs_path = Some(path.as_ref().to_path_buf());
        }
    }

    pub fn get_fs_path(&self) -> Option<&Path> {
        self.0.as_ref().and_then(|dinfo| {
            dinfo.fs_path.as_ref().map(|x| x as &Path)
        })
    }

    pub fn get_request_id(&self) -> Option<RequestId> {
        self.0.as_ref().map(|dinfo| dinfo.request_id)
    }

    pub fn add_allow<D: Display>(&mut self, s: D) {
        if let Some(ref mut dinfo) = self.0 {
            if dinfo.allow.len() > 0 {
                write!(&mut dinfo.allow, ", {}", s).unwrap();
            } else {
                write!(&mut dinfo.allow, "{}", s).unwrap();
            }
        }
    }

    pub fn get_allow(&self) -> Option<&str> {
        self.0.as_ref().and_then(|dinfo| {
            if dinfo.allow.len() == 0 {
                None
            } else {
                Some(&dinfo.allow[..])
            }
        })
    }

    pub fn set_deny<D: Display>(&mut self, s: D) {
        if let Some(ref mut dinfo) = self.0 {
            dinfo.deny = s.to_string();
        }
    }

    pub fn get_deny(&self) -> Option<&str> {
        self.0.as_ref().and_then(|dinfo| {
            if dinfo.deny.len() == 0 {
                None
            } else {
                Some(&dinfo.deny[..])
            }
        })
    }

    pub fn get_authorizer(&self) -> Option<&Authorizer> {
        self.0.as_ref().and_then(|dinfo| {
            dinfo.route.as_ref().map(|x| &x.authorizer_name)
        })
    }

}
