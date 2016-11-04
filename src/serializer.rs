use std::sync::Arc;
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

use futures::{BoxFuture, Future, Async, finished};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use tokio_service::Service;
use tk_bufstream::IoBuf;
use minihttp::{Error, GenericResponse, ResponseWriter, Status, Request};
use minihttp::enums::Method;
use minihttp::client::HttpClient;
use httpbin::HttpBin;

use config::{Config, EmptyGif};
use config::static_files::{Static, SingleFile};

use response::DebugInfo;
use default_error_page::error_page;
use handlers::{files, empty_gif, proxy};
use handlers::proxy::ProxyCall;
use chat;
use websocket;
use {Pickler};


pub struct Serializer {
    config: Arc<Config>,
    debug: DebugInfo,
    response: Response,
    handle: Remote,
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
    WebsocketChat(websocket::Init, HttpClient, chat::MessageRouter),
    Proxy(ProxyCall),
}

impl Response {
    pub fn serve(self, req: Request, cfg: Arc<Config>,
                 debug: DebugInfo, handle: &Handle,
                 http_client: &HttpClient)
        -> BoxFuture<Serializer, Error>
    {
        match self {
            Response::Proxy(ProxyCall::Prepare{ hostport, settings}) => {
                let handle = handle.remote().clone();

                let mut client = http_client.clone();
                proxy::prepare(req, hostport, settings, &mut client);

                client.done()
                    .map_err(|e| e.into())
                    .and_then(move |resp| {
                        let resp = Response::Proxy(ProxyCall::Ready {
                            response: resp,
                        });
                        finished(Serializer {
                            config: cfg,
                            debug: debug,
                            response: resp,
                            handle: handle,
                            request: None,
                        })
                    }).boxed()
            }
            Response::WebsocketChat(init, client, router) => {
                // TODO: issue Auth request to backend;
                //      if authorized — negotiate;
                //      otherwise — Response Error

                let mut auth = http_client.clone();
                let url = router.get_url("tangle.authorize_connection".into());
                auth.request(Method::Post, url.as_str());
                auth.done();
                finished(Serializer {
                    config: cfg,
                    debug: debug,
                    response: Response::WebsocketChat(init, client, router),
                    handle: handle.remote().clone(),
                }).boxed()
            }
            _ => {
                finished(Serializer {
                    config: cfg,
                    debug: debug,
                    response: self,
                    handle: handle.remote().clone(),
                    request: Some(req), // only needed for HttpBin
                }).boxed()
            }
        }
    }
}

impl<S: Io + AsRawFd + Send + 'static> GenericResponse<S> for Serializer {
    type Future = BoxFuture<IoBuf<S>, Error>;
    fn into_serializer(mut self, writer: ResponseWriter<S>) -> Self::Future {
        let writer = Pickler(writer, self.config, self.debug);
        match self.response {
            Response::ErrorPage(status) => {
                error_page(status, writer)
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
            Response::WebsocketChat(init, client, router) => {
                chat::negotiate(writer, init, self.handle, client, router)
            }
            Response::Proxy(ProxyCall::Ready { response }) => {
                proxy::serialize(writer, response)
            }
            Response::Proxy(_) => {
                error_page(Status::BadRequest, writer)
            }
        }
    }
}
