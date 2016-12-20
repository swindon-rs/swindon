//! TODO(tailhook) maybe unbundle this module into separate crate?
//!
use std::io;
use std::str::from_utf8;

use futures::{AsyncSink, Async, Future};
use futures::sink::Sink;

use rustc_serialize::json::{Json};
use tokio_core::io::Io;
use tokio_core::net::TcpStream;
use futures::sync::oneshot::{channel, Sender};
use futures::sync::mpsc::SendError;
use minihttp::client::{Error, Codec, Encoder, EncoderDone, Head, RecvMode};
use minihttp::OptFuture;

use http_pools::UpstreamRef;


pub struct JsonRequest<F> {
    request: Option<F>,
    sender: Option<Sender<Result<Json, Error>>>,
}

pub fn request_fn_buffered<F, E>(mut pool: UpstreamRef, f: F)
    -> OptFuture<Json, E>
    where F: FnOnce(Encoder<TcpStream>) -> EncoderDone<TcpStream> + Send + 'static,
          E: From<Error>,
          E: From<SendError<Box<Codec<TcpStream>+Send>>>,
{
    let (tx, rx) = channel();
    let codec = JsonRequest {
        request: Some(f),
        sender: Some(tx),
    };
    let mut guard = pool.get_mut();
    let ref mut pool = match guard.get_mut() {
        Some(pool) => pool,
        None => return OptFuture::Value(Err(Error::Busy.into())),
    };
    match pool.start_send(Box::new(codec)) {
        Ok(AsyncSink::NotReady(_)) => {
            OptFuture::Value(Err(Error::Busy.into()))
        }
        Ok(AsyncSink::Ready) => {
            OptFuture::Future(
                rx
                .map_err(|_| Error::Canceled.into())
                .and_then(|res| res)
                .map_err(|e| e.into())
                .boxed())
        }
        Err(e) => {
            OptFuture::Value(Err(e.into()))
        }
    }
}

impl<F, S: Io> Codec<S> for JsonRequest<F>
    where F: FnOnce(Encoder<S>) -> EncoderDone<S>
{
    fn start_write(&mut self, e: Encoder<S>)
        -> OptFuture<EncoderDone<S>, Error>
    {
        OptFuture::Value(Ok(self.request.take().expect("called once")(e)))
    }
    fn headers_received(&mut self, headers: &Head) -> Result<RecvMode, Error> {
        if headers.code != 200 {
            // TODO(tailhook) fix error
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::Other,
                "bad response code")));
        }
        Ok(RecvMode::Buffered(10_048_576))
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        assert!(end);
        let response = from_utf8(data)
            .map_err(|_| {
                // TODO(tailhook) fix error
                Error::Io(io::Error::new(io::ErrorKind::Other,
                    "bad response code"))
            })
            .and_then(|s| {
                Json::from_str(s)
                .map_err(|e| {
                    Error::Io(io::Error::new(io::ErrorKind::Other, e))
                })
            });
        self.sender.take().unwrap().complete(response);
        Ok(Async::Ready(data.len()))
    }
}
