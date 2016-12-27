use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::collections::HashMap;

use time;
use futures::{Finished};
use minihttp::server::{ResponseWriter, Error, Request};
use tokio_core::io::Io;
use tk_bufstream::{IoBuf, Flushed};


use config::Config;
use intern::HandlerName;


pub struct DebugInfo {
    route: Option<HandlerName>,
}


pub struct Pickler<S: Io>(pub ResponseWriter<S>,
                          pub Arc<Config>, pub DebugInfo);

impl<S: Io> Pickler<S> {
    pub fn add_length(&mut self, n: u64) {
        self.0.add_length(n).unwrap();
    }
    pub fn add_chunked(&mut self) {
        self.0.add_chunked().unwrap();
    }
    pub fn add_header<V: AsRef<[u8]>>(&mut self, name: &str, value: V) {
        self.0.add_header(name, value).unwrap();
    }
    pub fn format_header<D: Display>(&mut self, name: &str, value: D) {
        self.0.format_header(name, value).unwrap();
    }
    /// This adds headers specified by user in the configuration. I.e. it
    /// pretends to be fail-safe. But *may skip invalid header* with
    /// a warning.
    pub fn add_extra_headers(&mut self, headers: &HashMap<String, String>) {
        for (name, value) in headers {
            match self.0.add_header(name, value) {
                Ok(()) => {}
                Err(e) => {
                    warn!("Can't add header: {:?}:{:?}, reason {}. \
                        Almost always this means that something wrong with \
                        configuration of extra headers.",
                        name, value, e);
                }
            }
        }
    }
    pub fn steal_socket(self) -> Flushed<S> {
        let Pickler(wr, _cfg, _debug) = self;
        wr.steal_socket()
    }
    pub fn done_headers(&mut self) -> bool {
        let Pickler(ref mut wr, ref cfg, ref debug) = *self;
        cfg.server_name.as_ref().map(|name| {
            wr.add_header("Server", name).unwrap();
        });
        wr.format_header("Date", time::now().rfc822()).unwrap();
        if cfg.debug_routing {
            wr.add_header("X-Swindon-Route",
                debug.route.as_ref().map(|x| &x[..]).unwrap_or("-- none --")
            ).expect("route is a valid header");
        }

        wr.done_headers().unwrap()
    }
    pub fn done(self) -> Finished<IoBuf<S>, Error> {
        self.0.done()
    }
    pub fn debug_routing(&self) -> bool {
        self.1.debug_routing
    }
}

impl<S: Io> Deref for Pickler<S> {
    type Target = ResponseWriter<S>;
    fn deref(&self) -> &ResponseWriter<S> {
        &self.0
    }
}

impl<S: Io> DerefMut for Pickler<S> {
    fn deref_mut(&mut self) -> &mut ResponseWriter<S> {
        &mut self.0
    }
}
