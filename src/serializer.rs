use std::sync::Arc;

use netbuf::Buf;
use futures::{BoxFuture, Future, finished};
use tokio_core::net::TcpStream;
use minihttp::{Error, GenericResponse, ResponseWriter};

use config::Config;
use default_error_page::error_page;


pub struct Serializer {
    #[allow(dead_code)]
    config: Arc<Config>,
    response: Response,
}

pub enum Response {
    ErrorPage(u16),
}

impl Response {
    pub fn serve(self, cfg: Arc<Config>) -> BoxFuture<Serializer, Error> {
        finished(Serializer {
            config: cfg,
            response: self,
        }).boxed()
    }
}

impl GenericResponse for Serializer {
    type Future = BoxFuture<(TcpStream, Buf), Error>;
    fn into_serializer(self, writer: ResponseWriter) -> Self::Future {
        match self.response {
            Response::ErrorPage(code) => {
                // TODO(tailhook) resolve statuses
                error_page(code, "Unknown", writer)
            }
        }
    }
}
