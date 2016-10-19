use std::sync::Arc;
use std::path::PathBuf;

use netbuf::Buf;
use futures::{BoxFuture, Future, finished};
use tokio_core::net::TcpStream;
use minihttp::{Error, GenericResponse, ResponseWriter};

use config::{Config};
use config::static_files::Static;
use response::DebugInfo;
use default_error_page::error_page;
use handlers::{files, empty_gif};
use {Pickler};


pub struct Serializer {
    config: Arc<Config>,
    debug: DebugInfo,
    response: Response,
}

pub enum Response {
    ErrorPage(u16),
    EmptyGif,
    Static {
        path: PathBuf,
        settings: Arc<Static>,
    }
}

impl Response {
    pub fn serve(self, cfg: Arc<Config>, debug: DebugInfo)
        -> BoxFuture<Serializer, Error>
    {
        finished(Serializer {
            config: cfg,
            debug: debug,
            response: self,
        }).boxed()
    }
}

impl GenericResponse for Serializer {
    type Future = BoxFuture<(TcpStream, Buf), Error>;
    fn into_serializer(self, writer: ResponseWriter) -> Self::Future {
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
        }
    }
}
