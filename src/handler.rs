use std::sync::{Arc, RwLock};

use futures::{BoxFuture, Future};
use tokio_service::Service;
use tokio_core::reactor::Handle;
use minihttp::server::{Error, Request};
use minihttp::{Status, OptFuture};
use minihttp::client::Client;
use rand::{thread_rng, Rng};

use config::ConfigCell;
use response::DebugInfo;
use routing::{parse_host, route};
use serializer::{Response, Serializer};
use config::Handler;
use handlers::{files, proxy};
use intern::{HandlerName, Upstream};
use chat;
use websocket;
use http_pools::HttpPools;

#[derive(Clone)]
pub struct Main {
    pub config: ConfigCell,
    pub handle: Handle,
    pub http_pools: HttpPools,
    pub chat_processor: Arc<RwLock<chat::Processor>>,
}

impl Service for Main {
    type Request = Request;
    type Response = Serializer;
    type Error = Error;
    type Future = Box<Future<Item=Self::Response, Error=Error>>;

    fn call(&self, req: Request) -> Self::Future {
        // We must store configuration for specific request for the case
        // it changes in runtime. Config changes in the middle of request
        // can create undesirable effects
        let cfg = self.config.get();
        let mut debug = DebugInfo::new(&req);

        let response = self.prepare_response(&req, &mut debug);
        response.serve(req, cfg.clone(), debug, &self.handle, &self.http_pools)
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

    fn match_handler(&self, route: &HandlerName, suffix: &str, req: &Request)
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
                Response::Proxy(proxy::ProxyCall::Prepare {
                    path: format!("{}{}", settings.destination.path, suffix),
                    settings: settings.clone(),
                })
            }
            Some(&Handler::SwindonChat(ref chat)) => {
                match websocket::prepare(&req) {
                    Ok(init) => {
                        let pool = self.chat_processor.read().unwrap()
                            .pool(&chat.session_pool);
                        let sess_cfg = cfg.session_pools
                            .get(&chat.session_pool).unwrap(); // FIXME: unwrap
                        let chat_api = chat::ChatAPI::new(
                            self.http_pools.clone(), chat.clone(), pool,
                            sess_cfg.inactivity.clone());
                        Response::WebsocketChat(
                            chat::ChatInit::Prepare(init, chat_api))
                    }
                    Err(_) => {
                        // internal redirect
                        let ref route = chat.http_route;
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
