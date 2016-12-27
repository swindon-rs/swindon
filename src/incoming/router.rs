use std::net::SocketAddr;
use std::sync::Arc;

use tokio_core::io::Io;
use minihttp::server::{Dispatcher, Error, Head};

use config::ConfigCell;
use runtime::Runtime;
use incoming::{Request, Debug};
use routing::{parse_host, route};


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
        // Keep config same while processing a single request
        let cfg = self.runtime.config.get();
        let mut debug = Debug::new(headers, &cfg);

        // No path means either CONNECT host, or OPTIONS *
        // in both cases we use root route for the domain to make decision
        let path = headers.path().unwrap_or("/");

        let matched_route = headers.host().map(parse_host)
            .and_then(|host| route(host, &path, &cfg.routing));
        if let Some((route, suffix)) = matched_route {
            debug.set_route(route);
            println!("ROUTE {:?}", route);
            unimplemented!();
        } else {
            //Response::ErrorPage(Status::NotFound)
            unimplemented!();
        }
    }
}
