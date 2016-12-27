use std::net::SocketAddr;
use std::sync::Arc;

use tokio_core::io::Io;
use minihttp::server::{Dispatcher, Error, Head};

use config::ConfigCell;
use runtime::Runtime;
use incoming::Request;

pub struct Router {
    addr: SocketAddr,
    runtime: Arc<Runtime>,
}

impl Router {
    pub fn new(addr: SocketAddr, runtime: Arc<Runtime>) -> Router {
        Router {
            addr: addr,
            runtime: runtime,
        }
    }
}

impl<S: Io> Dispatcher<S> for Router {
    type Codec = Request<S>;
    fn headers_received(&mut self, headers: &Head)
        -> Result<Self::Codec, Error>
    {
        unimplemented!();
    }
}
