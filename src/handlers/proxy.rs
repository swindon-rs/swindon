use std::str;
use std::sync::Arc;

use futures::{BoxFuture, Future, Poll, Async};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use minihttp::{Error, Request, Status};
use minihttp::enums::{Method, Header};
use tk_bufstream::IoBuf;
use tokio_curl::{Session, Perform};
use curl::easy::{Easy, List};

use config::Config;
use config::proxy::Proxy;
use config::http_destinations::Destination;
use serializer::{Response, Serializer};
use response::DebugInfo;
use {Pickler};


pub enum ProxyCall {
    Prepare {
        hostport: String,
        settings: Arc<Proxy>,
        session: Session,
    },
    Ready(Easy),
}


pub struct CurlRequest {
    easy: Option<Easy>,
    session: Session,
    perform: Option<Perform>,
}

impl CurlRequest {
    pub fn new(request: Request,
               hostport: String, session: Session,
               settings: Arc<Proxy>)
    -> CurlRequest
    {
        let mut easy = Easy::new();
        easy.forbid_reuse(true).unwrap();

        match request.method {
            Method::Get => easy.get(true).unwrap(),
            Method::Post => easy.post(true).unwrap(),
            _ => panic!("Not implemented"),
        };
        easy.url(format!("http://{}{}", hostport, request.path).as_str())
            .unwrap();

        let mut headers = List::new();
        for &(ref name, ref value) in &request.headers {
            // TODO: validate headers
            let line = match name {
                &Header::Host => format!("Host: {}", value),
                &Header::Raw(ref name) => format!("{}: {}", name, value),
                _ => continue,
            };
            headers.append(line.as_str()).unwrap();
        }
        // TODO: add settings.ip_header to list;
        easy.http_headers(headers);

        // wrapper.easy.write_function(|buf| {
        //     println!("BODY: {}", buf.len());
        //     Ok(buf.len())
        // }).unwrap();
        // wrapper.easy.header_function(|h| {
        //     println!("Header: {:?}", str::from_utf8(h).unwrap());
        //     true
        // }).unwrap();

        let mut req = CurlRequest {
            session: session,
            easy: Some(easy),
            perform: None,
        };
        req.init();
        req
    }

    fn init(&mut self) {
    }

    pub fn into_future(self) -> BoxFuture<Easy, Error> {
        self.boxed()
    }
}

impl Future for CurlRequest {
    type Item = Easy;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let poll = match self.perform {
            Some(ref mut perform) => perform.poll(),
            None => {
                let mut perform = self.easy.take()
                    .map(|easy| self.session.perform(easy))
                    .unwrap();
                self.perform = Some(perform);
                self.perform.as_mut().unwrap().poll()
            }
        };
        match poll {
            Ok(Async::Ready(resp)) => {
                // self.easy = Some(resp);
                Ok(Async::Ready(resp))
            },
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e.into_error().into()),
        }
    }
}


pub fn pick_backend_host(dest: &Destination) -> String
{
    // TODO: poperly implement Destination pickup

    //match dest.load_balancing {
    //    LoadBalancing::queue => {}
    //}

    // Pick backend address;
    // XXX: currently pick first one:
    dest.addresses.first().unwrap().clone()
}


pub fn serve<S>(mut response: Pickler<S>, mut resp: Easy)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static,
{
    let code = resp.response_code().unwrap();
    // TODO: handle response codes respectively,
    //      ie 204 has no body.
    let status = Status::from(code as u16).unwrap();
    response.status(status);
    if status.response_has_body() {
        // TODO: add body
        response.add_length(0);
    }
    response.done_headers();
    response.done().boxed()
}
