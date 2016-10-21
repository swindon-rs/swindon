use std::sync::Arc;
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

use futures::{BoxFuture, Future, finished};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use tk_bufstream::IoBuf;
use minihttp::{Error, GenericResponse, ResponseWriter};
use tokio_curl::Session;

use config::{Config};
use config::static_files::Static;
// use config::proxy::Proxy;
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
    ErrorPage(u16),
    EmptyGif,
    Static {
        path: PathBuf,
        settings: Arc<Static>,
    },
    WebsocketEcho(websocket::Init),
    Proxy {
        session: Session,
        // settings: Arc<Proxy>,
        call: proxy::UpstreamCall,
    },
}

impl Response {
    pub fn serve(self, cfg: Arc<Config>, debug: DebugInfo, handle: &Handle)
        -> BoxFuture<Serializer, Error>
    {
        finished(Serializer {
            config: cfg,
            debug: debug,
            response: self,
            handle: handle.remote().clone(),
        }).boxed()
    }
}

impl<S: Io + AsRawFd + Send + 'static> GenericResponse<S> for Serializer {
    type Future = BoxFuture<IoBuf<S>, Error>;
    fn into_serializer(self, writer: ResponseWriter<S>) -> Self::Future {
        let writer = Pickler(writer, self.config, self.debug);
        match self.response {
            Response::ErrorPage(code) => {
                // TODO(tailhook) resolve statuses
                error_page(code, "Unknown", writer)
            }
            Response::EmptyGif => {
                empty_gif::serve(writer)
            }
            Response::Static { path, settings } => {
                files::serve(writer, path, settings)
            }
            Response::WebsocketEcho(init) => {
                websocket::negotiate(writer, init, self.handle)
            }
            Response::Proxy { session, call } => {
                // TODO(popravich) determine proxy destination and headers
                proxy::serve(writer, session, call)
            }
        }
    }
}
