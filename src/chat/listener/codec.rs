use std::sync::Arc;

use futures::{Async, Future};
use tokio_core::io::Io;
use tokio_core::reactor::Handle;
use minihttp::server::{Dispatcher, Error, Head};
use minihttp::server as http;
use minihttp::server::{EncoderDone, RecvMode, WebsocketAccept};

use config::SessionPool;
use runtime::Runtime;

pub struct Handler {
    runtime: Arc<Runtime>,
    settings: Arc<SessionPool>,
    handle: Handle,
}

pub enum Request {

}

pub enum Response<S> {
    TODO(S),
}

impl Handler {
    pub fn new(runtime: Arc<Runtime>, settings: Arc<SessionPool>,
        handle: Handle)
        -> Handler
    {
        Handler {
            runtime: runtime,
            settings: settings,
            handle: handle,
        }
    }
}

impl<S: Io> Dispatcher<S> for Handler {
    type Codec = Request;
    fn headers_received(&mut self, headers: &Head)
        -> Result<Self::Codec, Error>
    {
        unimplemented!();
    }
}

impl<S: Io> http::Codec<S> for Request {
    type ResponseFuture = Response<S>;
    fn recv_mode(&mut self) -> RecvMode {
        unimplemented!();
        //RecvMode::BufferedUpfront(self.settings.max_payload_size)
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        unimplemented!();
    }
    fn start_response(&mut self, e: http::Encoder<S>) -> Response<S> {
        unimplemented!();
    }
}

impl<S: Io> Future for Response<S> {
    type Item = EncoderDone<S>;
    type Error = Error;
    fn poll(&mut self) -> Result<Async<EncoderDone<S>>, Error> {
        unimplemented!();
    }
}
