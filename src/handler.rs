use futures::{Async, BoxFuture};
use tokio_service::Service;
use minihttp::request::Request;
use minihttp::Error;

use config::ConfigCell;
use routing::{parse_host, route};
use serializer::{Response, Serializer};

#[derive(Clone)]
pub struct Main {
    pub config: ConfigCell,
}

impl Service for Main {
    type Request = Request;
    type Response = Serializer;
    type Error = Error;
    type Future = BoxFuture<Self::Response, Error>;

    fn call(&self, req: Request) -> Self::Future {
        // We must store configuration for specific request for the case
        // it changes in runtime. Config changes in the middle of request
        // can create undesirable effects
        let cfg = self.config.get();

        if let Some(host) = req.host().map(parse_host) {
            if let Some(_route) = route(host, &req.path, &cfg.routing) {
                return Response::ErrorPage(404).serve(cfg.clone());
            }
        }
        Response::ErrorPage(404).serve(cfg)
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}
