//! Pull API handler.
use futures::{Async, Finished, finished};
use tokio_service::Service;
use tokio_core::net::TcpStream;
use tk_bufstream::IoBuf;
use minihttp::{Error, Request};
use minihttp::{ResponseFn, Status};

#[derive(Clone)]
pub struct ChatAPI;


impl Service for ChatAPI {
    type Request = Request;
    type Response = ResponseFn<Finished<IoBuf<TcpStream>, Error>, TcpStream>;
    type Error = Error;
    type Future = Finished<Self::Response, Error>;

    fn call(&self, req: Request) -> Self::Future {
        // TODO: match route;
        //  pick handler;
        //  make response;
        //  serialize response;
        finished(ResponseFn::new(move |mut res| {
            res.status(Status::NoContent);
            // TODO: add debug headers;
            res.done_headers().unwrap();
            res.done()
        }))
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}
