use futures::{Async, BoxFuture};
use tokio_service::Service;
use minihttp::request::Request;
use minihttp::Error;

use config::ConfigCell;
use response::DebugInfo;
use routing::{parse_host, route};
use serializer::{Response, Serializer};
use config::Handler;

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
        let cfg2 = cfg.clone();
        let mut debug = DebugInfo::new(&req);

        if let Some(host) = req.host().map(parse_host) {
            if let Some(route) = route(host, &req.path, &cfg.routing) {
                debug.set_route(route);
                match cfg.handlers.get(route) {
                    Some(&Handler::EmptyGif) => {
                        return Response::EmptyGif.serve(cfg2, debug)
                    }
                    // TODO(tailhook) make better error code for None
                    _ => {
                        // Not implemented
                        return Response::ErrorPage(501).serve(cfg2, debug)
                    }
                }
            }
        }
        Response::ErrorPage(404).serve(cfg, debug)
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}
