use futures::{Async, BoxFuture};
use tokio_service::Service;
use minihttp::request::Request;
use minihttp::Error;

use config::ConfigCell;
use response::DebugInfo;
use routing::{parse_host, route};
use serializer::{Response, Serializer};
use config::Handler;
use handlers::files;

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

        let matched_route = req.host().map(parse_host)
            .and_then(|host| route(host, &req.path, &cfg.routing));
        if let Some((route, suffix)) = matched_route {
            debug.set_route(route);
            match cfg.handlers.get(route) {
                Some(&Handler::EmptyGif) => {
                    Response::EmptyGif.serve(cfg2, debug)
                }
                Some(&Handler::Static(ref settings)) => {
                    if let Ok(path) = files::path(settings, suffix, &req) {
                        Response::Static {
                            path: path,
                            settings: settings.clone(),
                        }.serve(cfg2, debug)
                    } else {
                        Response::ErrorPage(403).serve(cfg2, debug)
                    }
                }
                // TODO(tailhook) make better error code for None
                _ => {
                    // Not implemented
                    Response::ErrorPage(501).serve(cfg2, debug)
                }
            }
        } else {
            Response::ErrorPage(404).serve(cfg2, debug)
        }
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}
