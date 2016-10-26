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
    Proxy(proxy::ProxyCall),
}

impl Response {
    pub fn serve(self, req: Request, cfg: Arc<Config>,
                 debug: DebugInfo, handle: &Handle,
                 curl_session: &Session)
        -> BoxFuture<Serializer, Error>
    {
        use handlers::proxy::ProxyCall::*;
        match self {
            Response::Proxy(Prepare{ hostport, settings}) => {
                let handle = handle.remote().clone();
                let resp_buf = Arc::new(Mutex::new(Buf::new()));
                let headers_counter = Arc::new(Mutex::new(0));

                let request = proxy::prepare(
                    req, hostport, settings,
                    resp_buf.clone(),
                    headers_counter.clone())
                    .unwrap();

                curl_session.perform(request)
                    .map_err(|err| err.into_error().into())
                    .and_then(move |resp| {
                        let body = resp_buf.lock().unwrap().split_off(0);
                        let n = headers_counter.lock().unwrap().clone();
                        let resp = Response::Proxy(
                            Ready(resp, n, body));
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
            Response::Proxy(state) => {
                match state {
                    proxy::ProxyCall::Ready(req, num_headers, resp_buf) => {
                        proxy::serve(writer, req, num_headers, resp_buf)
                    }
                    _ => panic!("Unreachable state")
                }
            }
        }
    }
}

impl Serializer {
    pub fn new(config: Arc<Config>, debug: DebugInfo,
               response: Response, handle: Remote)
        -> Serializer
    {
        Serializer {
            config: config,
            debug: debug,
            response: response,
            handle: handle,
        }
    }
}
