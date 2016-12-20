use std::str;
use std::sync::Arc;
use std::ascii::AsciiExt;
use std::convert::From;

use futures::{AsyncSink, Async, Future, };
use futures::sink::Sink;
use futures::sync::mpsc::SendError;
use tokio_core::io::Io;
use tokio_core::net::TcpStream;
use minihttp::server::{Request, Error as HttpError};
use minihttp::OptFuture;
use minihttp::enums::{Header, Version};
use minihttp::client::{Error, Codec, Encoder, EncoderDone, Head, RecvMode};
use futures::sync::oneshot::{channel, Sender};
use tk_bufstream::IoBuf;

use http_pools::UpstreamRef;
use config::proxy::Proxy;
use {Pickler};

pub struct ProxyCodec {
    settings: Arc<Proxy>,
    request: Option<(Request, String)>,
    response: Option<Response>,
    // Temporarily buffering everything
    sender: Option<Sender<Result<Response, Error>>>,
}

pub enum ProxyCall {
    Prepare {
        path: String,
        settings: Arc<Proxy>,
    },
    Ready {
        response: Response,
    },
}

#[derive(Debug)]
/// A buffered response holds contains a body as contiguous chunk of data
pub struct Response {
    code: u16,
    reason: String,
    headers: Vec<(String, Vec<u8>)>,
    body: Vec<u8>,
}


/// Serialize buffered response.
///
pub fn serialize<S>(mut e: Pickler<S>, resp: Response)
    -> Box<Future<Item=IoBuf<S>, Error=HttpError>>
    where S: Io + Send + 'static,
{
    // TODO: handle response codes respectively,
    //      ie 204 has no body.
    e.custom_status(resp.code, &resp.reason);
    for (k, v) in resp.headers {
        // TODO(tailhook) skip connection/hop-by-hop headers
        // TODO(tailhook) skip Server headers
        if k.eq_ignore_ascii_case("Content-Length") &&
           k.eq_ignore_ascii_case("Transfer-Endoding")
        {
            e.add_header(&k, v);
        }
    }
    e.add_length(resp.body.len() as u64);
    if e.done_headers() {
        e.write_body(&resp.body);
    }
    e.done().boxed()
}

pub fn request<E>(mut pool: UpstreamRef, settings: Arc<Proxy>,
    path: String, req: Request)
    -> OptFuture<Response, E>
    where
          E: From<Error>,
          E: From<SendError<Box<Codec<TcpStream>+Send>>>,
{
    let (tx, rx) = channel();
    let codec = ProxyCodec {
        settings: settings,
        request: Some((req, path)),
        response: None,
        sender: Some(tx),
    };
    let mut guard = pool.get_mut();
    let pool = match guard.get_mut() {
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

impl<S: Io> Codec<S> for ProxyCodec {
    fn start_write(&mut self, mut e: Encoder<S>)
        -> OptFuture<EncoderDone<S>, Error>
    {
        let ref cfg = self.settings;
        let (req, path) = self.request.take()
            .expect("request serialized twice?");
        e.request_line(req.method.as_ref(), &path, Version::Http11);


        if let Some(ref ip_header) = cfg.ip_header {
            e.format_header(&ip_header[..], req.peer_addr.ip()).unwrap();
        }

        for &(ref name, ref value) in &req.headers {
            // TODO(tailhook) skip connection headers
            match name {
                &Header::Host => e.add_header("Host", value).unwrap(),
                &Header::Raw(ref name) => e.add_header(name, value).unwrap(),
                // TODO(tailhook) why we skip all other?
                _ => continue,
            };
        }
        if let Some(body) = req.body {
            e.add_length(body.data.len() as u64).unwrap();
            e.done_headers().unwrap();
            e.write_body(&body.data[..]);
        } else {
            e.done_headers().unwrap();
        }
        OptFuture::Value(Ok(e.done()))
    }
    fn headers_received(&mut self, headers: &Head) -> Result<RecvMode, Error> {
        self.response = Some(Response {
            code: headers.code,
            reason: headers.reason.to_string(),
            headers: headers.headers.iter().map(|&header| {
                (header.name.to_string(), header.value.to_vec())
            }).collect(),
            body: Vec::new(),
        });
        // TODO(tailhook) lift this limit, probably when progressive download
        // is implemented
        Ok(RecvMode::Buffered(10_048_576))
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        assert!(end);
        let mut response = self.response.take().unwrap();
        response.body = data.to_vec();
        self.sender.take().unwrap().complete(Ok(response));
        Ok(Async::Ready(data.len()))
    }
}
