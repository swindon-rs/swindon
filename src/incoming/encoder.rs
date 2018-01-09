use std::collections::HashMap;
use std::fmt::Display;
use std::io;
use std::sync::Arc;

use futures::{Future, Async};
use tk_http::Status;
use tk_http::server as http;
use tk_http::server::{EncoderDone};
use tokio_io::AsyncWrite;


use config::Config;
use incoming::Debug;

pub type Context = (Arc<Config>, Debug);


pub struct Encoder<S> {
    enc: http::Encoder<S>,
    config: Arc<Config>,
    debug: Debug,
}

pub struct WaitFlush<S> {
    fut: http::WaitFlush<S>,
    data: Option<(Arc<Config>, Debug)>,
}

/// Represents object that can be used for getting enough context for encoder
/// from
pub trait IntoContext: Sized {
    fn into_context(self) -> Context;
}

impl IntoContext for (Arc<Config>, Debug) {
    fn into_context(self) -> Context {
        self
    }
}

impl<S: AsyncWrite> Future for WaitFlush<S> {
    type Item = Encoder<S>;
    type Error = io::Error;
    fn poll(&mut self) -> Result<Async<Encoder<S>>, io::Error> {
        match self.fut.poll()? {
            Async::Ready(x) => {
                let (config, debug) = self.data.take()
                    .expect("future polled twice");
                Ok(Async::Ready(Encoder {
                    enc: x,
                    config: config,
                    debug: debug,
                }))
            }
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}

impl<S> Encoder<S> {
    pub fn new(enc: http::Encoder<S>, context: Context)
        -> Encoder<S>
    {
        let (config, debug) = context;
        Encoder {
            enc: enc,
            config: config,
            debug: debug,
        }
    }
}

impl<S> Encoder<S> {
    pub fn status(&mut self, status: Status) {
        self.enc.status(status);
    }
    pub fn custom_status(&mut self, code: u16, reason: &str) {
        self.enc.custom_status(code, reason);
    }
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
        enc.add_date();
        if let Some(route) = self.debug.get_route() {
            enc.add_header("X-Swindon-Route", route)
                .expect("route is a valid header");
        }
        if let Some(path) = self.debug.get_fs_path() {
            enc.format_header("X-Swindon-File-Path",
                              format_args!("{:?}", path))
                .map_err(|e| error!("Adding X-Swindon-File-Path: {}", e)).ok();
        }
        if let Some(rid) = self.debug.get_request_id() {
            enc.format_header("X-Swindon-Request-Id", rid)
                .expect("request id valid");
        }
        if let Some(value) = self.debug.get_authorizer() {
            enc.format_header("X-Swindon-Authorizer", value)
                .expect("authorizer is a valid header");
        }
        if let Some(value) = self.debug.get_allow() {
            enc.format_header("X-Swindon-Allow", value)
                .expect("allow debug info is a valid header");
        }
        if let Some(value) = self.debug.get_deny() {
            enc.format_header("X-Swindon-Deny", value)
                .expect("deny debug info is a valid header");
        }

        enc.done_headers().unwrap()
    }
    pub fn write_body<T: AsRef<[u8]>>(&mut self, val: T) {
        self.enc.write_body(val.as_ref())
    }
    pub fn done(self) -> EncoderDone<S> {
        self.enc.done()
    }
    pub fn wait_flush(self, n: usize) -> WaitFlush<S> {
        WaitFlush {
            fut: self.enc.wait_flush(n),
            data: Some((self.config, self.debug)),
        }
    }
}

impl<S> io::Write for Encoder<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.enc.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.enc)
    }
}
