use std::io;

use futures::{Future, Async, finished, BoxFuture};
use tokio_service::Service;
use minihttp::request::Request;
use minihttp::response::Response;

use config::ConfigCell;
use routing::{parse_host, route};

#[derive(Clone)]
pub struct Main {
    pub config: ConfigCell,
}

impl Service for Main {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = BoxFuture<Response, io::Error>;

    fn call(&self, req: Request) -> Self::Future {
        // We must store configuration for specific request for the case
        // it changes in runtime. Config changes in the middle of request
        // can create undesirable effects
        let cfg = self.config.get();

        if let Some(host) = req.host().map(parse_host) {
            if let Some(route) = route(host, &req.path, &cfg.routing) {
                let mut resp = req.new_response();
                resp.set_status(200)
                    .set_reason("OK".into());
                if cfg.debug_routing {
                    resp.header("X-Swindon-Route", route);
                }
                resp.header("Content-Length", "0");
                return finished(resp).boxed();
            }
        }
        let mut resp = req.new_response();
        resp.set_status(404)
            .set_reason("Not Found".to_string())
            .header("Content-Length", "0");
        finished(resp).boxed()
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}
