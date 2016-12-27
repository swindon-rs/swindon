use std::sync::Arc;

use minihttp::server::Head;

use intern::HandlerName;
use config::Config;

pub struct Debug(Option<Box<DebugInfo>>);

struct DebugInfo {
    route: Option<HandlerName>,
    config: Arc<Config>,
}

impl Debug {
    pub fn new(head: &Head, cfg: &Arc<Config>) -> Debug {
        if cfg.debug_routing {
            Debug(Some(Box::new(DebugInfo {
                route: None,
                config: cfg.clone(),
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
    pub fn set_route(&mut self, route: &HandlerName) {
        if let Some(ref mut dinfo) = self.0 {
            debug_assert!(dinfo.route.is_none());
            dinfo.route = Some(route.clone());
        }
    }

    pub fn get_route(&self) -> Option<&str> {
        self.0.as_ref().map(|dinfo| {
            dinfo.route.as_ref().map(|x| &x[..])
            .unwrap_or("-- no route --")
        })
    }
}
