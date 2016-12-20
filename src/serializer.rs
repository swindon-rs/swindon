use std::sync::Arc;
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

use futures::{Future, Async, finished};
use futures::sync::mpsc::{unbounded as channel};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle};
use tokio_service::Service;
use tk_bufstream::IoBuf;
use minihttp::{client, Status};
use minihttp::server::{Error, GenericResponse, ResponseWriter, Request};
use httpbin::HttpBin;

use config::{Config, EmptyGif};
use config::static_files::{Static, SingleFile};

use http_pools::HttpPools;
use response::DebugInfo;
use default_error_page::write_error_page;
use handlers::{files, empty_gif, proxy};
use handlers::proxy::ProxyCall;
use chat;
use websocket;
use {Pickler};


pub struct Serializer {
    config: Arc<Config>,
    debug: DebugInfo,
    response: Response,
    handle: Handle,
    request: Option<Request>,
}

pub enum Response {
    ErrorPage(Status),
    EmptyGif(Arc<EmptyGif>),
    HttpBin,
    Static {
        path: PathBuf,
        settings: Arc<Static>,
    },
    SingleFile(Arc<SingleFile>),
    WebsocketEcho(websocket::Init),
    WebsocketChat(chat::ChatInit),
    Proxy(ProxyCall),
}

impl Response {
    pub fn serve(self, req: Request, cfg: Arc<Config>,
                 debug: DebugInfo, handle: &Handle,
                 http_client: &HttpPools)
        -> Box<Future<Item=Serializer, Error=Error>>
    {
        use self::Response::*;
        use super::chat::ChatInit::*;
        match self {
            Proxy(ProxyCall::Prepare { path, settings }) => {
                let h1 = handle.clone();
                let us = http_client.upstream(&settings.destination.upstream);
                Box::new(
                    proxy::request(us, settings.clone(), path, req)
                    .map_err(|e: client::Error| -> Error { unimplemented!() })
                    .map(move |resp| Serializer {
                        config: cfg,
                        debug: debug,
                        response: Proxy(ProxyCall::Ready { response: resp }),
                        handle: h1,
                        request: None,
                    }))
            }
            WebsocketChat(Prepare(init, mut chat_api)) => {
                let cid = chat::Cid::new();
                let (tx, rx) = channel();
                let h1 = handle.clone();

                Box::new(chat_api.authorize_connection(&req, cid, tx.clone())
                    .map_err(|e| -> Error { unimplemented!() })
                    .map(move |data| {
                        let resp = match chat::parse_userinfo(data) {
                            Ok((sess_id, userinfo)) => {
                                let session_api = chat_api.session_api(
                                    sess_id, cid, userinfo, tx);
                                WebsocketChat(Ready(init, session_api, rx))
                            }
                            Err(e) => {
                                error!("Bad data returned by \
                                    authorize_connection: {}", e);
                                WebsocketChat(AuthError(init,
                                    Status::InternalServerError))
                            }
                        };
                        Serializer {
                            config: cfg,
                            debug: debug,
                            response: resp,
                            handle: h1,
                            request: None,
                        }
                    }))
            }
            _ => {
                Box::new(finished(Serializer {
                    config: cfg,
                    debug: debug,
                    response: self,
                    handle: handle.clone(),
                    request: Some(req), // only needed for HttpBin
                }))
            }
        }
    }
}

impl<S: Io + AsRawFd + Send + 'static> GenericResponse<S> for Serializer {
    type Future = Box<Future<Item=IoBuf<S>, Error=Error>>;
    fn into_serializer(mut self, writer: ResponseWriter<S>) -> Self::Future {
        use super::chat::ChatInit::*;
        let writer = Pickler(writer, self.config, self.debug);
        match self.response {
            Response::ErrorPage(status) => {
                write_error_page(status, writer).done().boxed()
            }
            Response::EmptyGif(cfg) => {
                empty_gif::serve(writer, cfg)
            }
            Response::HttpBin => {
                // TODO(tailhook) it's not very good idea to unpack the future
                // this way
                match HttpBin::new().call(self.request.take().unwrap()).poll()
                {
                    Ok(Async::Ready(gen_response)) => {
                        gen_response.into_serializer(writer.0).boxed()
                    }
                    _ => unreachable!(),
                }
            }
            Response::Static { path, settings } => {
                files::serve(writer, path, settings)
            }
            Response::SingleFile(settings) => {
                files::serve_file(writer, settings)
            }
            Response::WebsocketEcho(init) => {
                websocket::negotiate(writer, init, self.handle,
                    websocket::Kind::Echo)
            }
            Response::WebsocketChat(Prepare(..)) => {
                unreachable!();
            }
            Response::WebsocketChat(Ready(init, session_api, rx)) =>
            {
                chat::negotiate(writer, init, self.handle, session_api, rx)
            }
            Response::WebsocketChat(AuthError(init, code)) => {
                chat::fail(writer, init, self.handle,
                    websocket::CloseReason::AuthHttp(code.code()))
            }
            Response::Proxy(ProxyCall::Ready { response }) => {
                proxy::serialize(writer, response)
            }
            Response::Proxy(_) => {
                write_error_page(Status::BadRequest, writer)
                .done().boxed()
            }
        }
    }
}
