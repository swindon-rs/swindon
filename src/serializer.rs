use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

use futures::{BoxFuture, Future, finished};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use tk_bufstream::IoBuf;
use minihttp::{Error, GenericResponse, ResponseWriter, Status, Request};
use netbuf::Buf;

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
                 debug: DebugInfo, handle: &Handle)
        -> BoxFuture<Serializer, Error>
    {
        use handlers::proxy::ProxyCall::*;
        match self {
            Response::Proxy(Prepare{ hostport, settings, session}) => {
                let handle = handle.remote().clone();
                let resp_buf = Arc::new(Mutex::new(Buf::new()));

                let easy = proxy::prepare(
                    req, hostport, settings, resp_buf.clone());

                session.perform(easy)
                    .map_err(|err| err.into_error().into())
                    .and_then(move |resp| {
                        finished(Serializer {
                            config: cfg,
                            debug: debug,
                            response: Response::Proxy(Ready(resp, resp_buf)),
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
                    proxy::ProxyCall::Ready(req, resp_buf) => {
                        proxy::serve(writer, req, resp_buf)
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
