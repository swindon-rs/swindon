use std::sync::Arc;

use minihttp::server::Head;

use intern::HandlerName;
use config::Config;

pub struct Debug(Option<Box<DebugInfo>>);

struct DebugInfo {
    handler: Option<HandlerName>,
    config: Arc<Config>,
}

impl Debug {
    pub fn new(head: &Head, cfg: &Arc<Config>) -> Debug {
        if cfg.debug_routing {
            Debug(Some(Box::new(DebugInfo {
                handler: None,
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
            debug_assert!(dinfo.handler.is_none());
            dinfo.handler = Some(route.clone());
        }
    }
}
