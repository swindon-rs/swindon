use futures::{Async, BoxFuture};
use tokio_service::Service;
use tokio_core::reactor::Handle;
use minihttp::request::Request;
use minihttp::Error;
use tokio_curl::Session;

use config::ConfigCell;
use response::DebugInfo;
use routing::{parse_host, route};
use serializer::{Response, Serializer};
use config::Handler;
use handlers::{files, proxy};
use websocket;

#[derive(Clone)]
pub struct Main {
    pub config: ConfigCell,
    pub handle: Handle,
    pub curl_session: Session,
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
        let mut debug = DebugInfo::new(&req);

        let matched_route = req.host().map(parse_host)
            .and_then(|host| route(host, &req.path, &cfg.routing));
        let response = if let Some((route, suffix)) = matched_route {
            debug.set_route(route);
            match cfg.handlers.get(route) {
                Some(&Handler::EmptyGif) => {
                    Response::EmptyGif
                }
                Some(&Handler::Static(ref settings)) => {
                    if let Ok(path) = files::path(settings, suffix, &req) {
                        Response::Static {
                            path: path,
                            settings: settings.clone(),
                        }
                    } else {
                        Response::ErrorPage(403)
                    }
                }
                Some(&Handler::WebsocketEcho) => {
                    match websocket::prepare(&req) {
                        Ok(init) => {
                            Response::WebsocketEcho(init)
                        }
                        Err(status) => {
                            // TODO(tailhook) use real status
                            Response::ErrorPage(
                                ::minihttp::enums::HttpStatus::code(&status)
                            )
                        }
                    }
                }
                Some(&Handler::Proxy(ref settings)) => {
                    if let Some(dest) = cfg.http_destinations.get(&settings.destination.upstream) {
                        match proxy::prepare(&req, &dest, settings.clone()) {
                            Ok(call) => {
                                Response::Proxy {
                                    session: self.curl_session.clone(),
                                    call: call,
                                }.serve(cfg2, debug)
                            }
                            Err(_) => {
                                Response::ErrorPage(500).serve(cfg2, debug)
                            }
                        }
                    } else {
                        Response::ErrorPage(404).serve(cfg2, debug)
                    }
                }
                // TODO(tailhook) make better error code for None
                _ => {
                    // Not implemented
                    Response::ErrorPage(501)
                }
            }
        } else {
            Response::ErrorPage(404)
        };
        response.serve(cfg.clone(), debug, &self.handle)
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}
