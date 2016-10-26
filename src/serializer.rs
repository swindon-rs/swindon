use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

use futures::{BoxFuture, Future, finished};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use tk_bufstream::IoBuf;
use minihttp::{Error, GenericResponse, ResponseWriter, Status, Request};
use netbuf::Buf;
use tokio_curl::Session;

use config::{Config};
use config::static_files::{Static, SingleFile};

use response::DebugInfo;
use default_error_page::error_page;
use handlers::{files, empty_gif, proxy};
use handlers::proxy::ProxyCall;
use websocket;
use {Pickler};


pub struct Serializer {
    config: Arc<Config>,
    debug: DebugInfo,
    response: Response,
    handle: Remote,
}

pub enum Response {
    ErrorPage(Status),
    EmptyGif,
    Static {
        path: PathBuf,
        settings: Arc<Static>,
    },
    SingleFile(Arc<SingleFile>),
    WebsocketEcho(websocket::Init),
    WebsocketChat(websocket::Init),
    Proxy(ProxyCall),
}

impl Response {
    pub fn serve(self, req: Request, cfg: Arc<Config>,
                 debug: DebugInfo, handle: &Handle,
                 curl_session: &Session)
        -> BoxFuture<Serializer, Error>
    {
        match self {
            Response::Proxy(ProxyCall::Prepare{ hostport, settings}) => {
                let handle = handle.remote().clone();
                let resp_buf = Arc::new(Mutex::new(Buf::new()));
                let num_headers = Arc::new(Mutex::new(0));

                let request = proxy::prepare(
                    req, hostport, settings,
                    resp_buf.clone(),
                    num_headers.clone())
                    .unwrap();

                curl_session.perform(request)
                    .map_err(|err| err.into_error().into())
                    .and_then(move |resp| {
                        let resp = Response::Proxy(ProxyCall::Ready {
                            curl: resp,
                            num_headers: num_headers.lock().unwrap().clone(),
                            body: resp_buf.lock().unwrap().split_off(0),
                        });
                        finished(Serializer {
                            config: cfg,
                            debug: debug,
                            response: resp,
                            handle: handle,
                        })
                    }).boxed()
            }
            _ => {
                finished(Serializer {
                    config: cfg,
                    debug: debug,
                    response: self,
                    handle: handle.remote().clone(),
                }).boxed()
            }
        }
    }
}

impl<S: Io + AsRawFd + Send + 'static> GenericResponse<S> for Serializer {
    type Future = BoxFuture<IoBuf<S>, Error>;
    fn into_serializer(self, writer: ResponseWriter<S>) -> Self::Future {
        let writer = Pickler(writer, self.config, self.debug);
        match self.response {
            Response::ErrorPage(status) => {
                error_page(status, writer)
            }
            Response::EmptyGif => {
                empty_gif::serve(writer)
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
            Response::WebsocketChat(init) => {
                websocket::negotiate(writer, init, self.handle,
                    websocket::Kind::SwindonChat)
            }
            Response::Proxy(ProxyCall::Ready {curl, num_headers, body }) => {
                proxy::serialize(writer, curl, num_headers, body)
            }
            Response::Proxy(_) => {
                error_page(Status::BadRequest, writer)
            }
        }
    }
}
