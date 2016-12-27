use std::fmt::Display;
use std::sync::Arc;
use std::collections::HashMap;

use time;
use minihttp::server as http;
use minihttp::server::{EncoderDone};
use tokio_core::io::Io;


use config::Config;
use incoming::Debug;


pub struct Encoder<S: Io> {
    enc: http::Encoder<S>,
    config: Arc<Config>,
    debug: Debug,
}

impl<S: Io> Encoder<S> {
    fn new(enc: http::Encoder<S>, config: Arc<Config>, debug: Debug)
        -> Encoder<S>
    {
        Encoder {
            enc: enc,
            config: config,
            debug: debug,
        }
    }
}

impl<S: Io> Encoder<S> {
    pub fn add_length(&mut self, n: u64) {
        self.enc.add_length(n).unwrap();
    }
    pub fn add_chunked(&mut self) {
        self.enc.add_chunked().unwrap();
    }
    pub fn add_header<V: AsRef<[u8]>>(&mut self, name: &str, value: V) {
        self.enc.add_header(name, value).unwrap();
    }
    pub fn format_header<D: Display>(&mut self, name: &str, value: D) {
        self.enc.format_header(name, value).unwrap();
    }
    /// This adds headers specified by user in the configuration. I.e. it
    /// pretends to be fail-safe. But *may skip invalid header* with
    /// a warning.
    pub fn add_extra_headers(&mut self, headers: &HashMap<String, String>) {
        for (name, value) in headers {
            match self.enc.add_header(name, value) {
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
    pub fn done_headers(&mut self) -> bool {
        let ref mut enc = self.enc;
        self.config.server_name.as_ref().map(|name| {
            enc.add_header("Server", name).unwrap();
        });
        enc.format_header("Date", time::now().rfc822()).unwrap();
        if let Some(route) = self.debug.get_route() {
            enc.add_header("X-Swindon-Route", route)
                .expect("route is a valid header");
        }

        enc.done_headers().unwrap()
    }
    pub fn done(self) -> EncoderDone<S> {
        self.enc.done()
    }
}
