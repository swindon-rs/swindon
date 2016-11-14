use futures::{Async, BoxFuture};
use tokio_service::Service;
use tokio_core::reactor::Handle;
use minihttp::request::Request;
use minihttp::{Error, Status};
use minihttp::client::HttpClient;

use config::ConfigCell;
use response::DebugInfo;
use routing::{parse_host, route};
use serializer::{Response, Serializer};
use config::Handler;
use handlers::{files, proxy};
use intern::Atom;
use chat::{self, MessageRouter};
use websocket;

#[derive(Clone)]
pub struct Main {
    pub config: ConfigCell,
    pub handle: Handle,
    pub http_client: HttpClient,
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

        let response = self.prepare_response(&req, &mut debug);
        response.serve(req, cfg.clone(), debug,
                       &self.handle, &self.http_client)
    }
}

impl Main {

    fn prepare_response(&self, req: &Request, debug: &mut DebugInfo)
        -> Response
    {
        let cfg = self.config.get();
        let matched_route = req.host().map(parse_host)
            .and_then(|host| route(host, &req.path, &cfg.routing));
        if let Some((route, suffix)) = matched_route {
            debug.set_route(route);
            // NOTE(popravich) debug.route may change when handler is matched;
            //  eg: ws chat route may fallback to some proxy route;
            self.match_handler(route, suffix, req)
        } else {
            Response::ErrorPage(Status::NotFound)
        }
    }

    fn match_handler(&self, route: &Atom, suffix: &str, req: &Request)
        -> Response
    {
        let cfg = self.config.get();
        match cfg.handlers.get(route) {
            Some(&Handler::EmptyGif(ref cfg)) => {
                Response::EmptyGif(cfg.clone())
            }
            Some(&Handler::HttpBin) => {
                Response::HttpBin
            }
            Some(&Handler::Static(ref settings)) => {
                if let Ok(path) = files::path(settings, suffix, &req) {
                    Response::Static {
                        path: path,
                        settings: settings.clone(),
                    }
                } else {
                    Response::ErrorPage(Status::Forbidden)
                }
            }
            Some(&Handler::SingleFile(ref settings)) => {
                Response::SingleFile(settings.clone())
            }
            Some(&Handler::WebsocketEcho) => {
                match websocket::prepare(&req) {
                    Ok(init) => {
                        Response::WebsocketEcho(init)
                    }
                    Err(status) => {
                        // TODO(tailhook) use real status
                        Response::ErrorPage(status)
                    }
                }
            }
            Some(&Handler::Proxy(ref settings)) => {
                if let Some(dest) = cfg.http_destinations
                        .get(&settings.destination.upstream)
                {
                    // NOTE: use suffix as real path?
                    let addr = dest.addresses.first().unwrap().clone();
                    Response::Proxy(proxy::ProxyCall::Prepare {
                        hostport: addr,
                        settings: settings.clone(),
                    })
                } else {
                    Response::ErrorPage(Status::NotFound)
                }
            }
            Some(&Handler::SwindonChat(ref chat)) => {
                match websocket::prepare(&req) {
                    Ok(init) => {
                        use super::chat::ChatInit::Prepare;
                        let router = MessageRouter(chat.clone(), cfg.clone());
                        Response::WebsocketChat(Prepare(init, router))
                    }
                    Err(_) => {
                        // internal redirect
                        let ref route = chat.http_route.upstream;
                        self.match_handler(route, suffix, req)
                    }
                }
            }
            // TODO(tailhook) make better error code for None
            None => {
                Response::ErrorPage(Status::NotFound)
            }
        }
    }
}
