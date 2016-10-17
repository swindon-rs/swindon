use std::io;

use futures::{Async, Finished, finished};
use tokio_service::Service;
use minihttp::request::Request;
use minihttp::response::Response;

use config::ConfigCell;

#[derive(Clone)]
pub struct Main {
    pub config: ConfigCell,
}

impl Service for Main {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = Finished<Response, io::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let mut resp = req.new_response();
        resp.set_status(204)
            .set_reason("No Content".to_string())
            .header("Content-Length", "0");
        finished(resp)
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}
