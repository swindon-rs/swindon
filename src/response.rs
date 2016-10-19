use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use netbuf::Buf;
use futures::{Finished};
use tokio_core::net::TcpStream;

use minihttp::{ResponseWriter, Error};

use config::Config;


pub struct Pickler(pub ResponseWriter, pub Arc<Config>);

impl Pickler {
    pub fn add_length(&mut self, n: u64) {
        self.0.add_length(n).unwrap();
    }
    pub fn add_chunked(&mut self) {
        self.0.add_chunked().unwrap();
    }
    pub fn add_header<V: AsRef<[u8]>>(&mut self, name: &str, value: V) {
        self.0.add_header(name, value).unwrap();
    }
    pub fn done_headers(&mut self) -> bool {
        let Pickler(ref mut wr, ref cfg) = *self;
        cfg.server_name.as_ref().map(|name| {
            wr.add_header("Server", name).unwrap();
        });
        wr.done_headers().unwrap()
    }
    pub fn done(self) -> Finished<(TcpStream, Buf), Error> {
        self.0.done()
    }
}

impl Deref for Pickler {
    type Target = ResponseWriter;
    fn deref(&self) -> &ResponseWriter {
        &self.0
    }
}

impl DerefMut for Pickler {
    fn deref_mut(&mut self) -> &mut ResponseWriter {
        &mut  self.0
    }
}
