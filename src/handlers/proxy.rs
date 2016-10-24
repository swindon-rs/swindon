use std::str;
use std::sync::{Arc, Mutex};
use std::io::Write;

use futures::{BoxFuture, Future, Poll, Async};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use minihttp::{Error, Request, Status};
use minihttp::enums::{Method, Header};
use tk_bufstream::IoBuf;
use tokio_curl::{Session, Perform};
use curl::easy::{Easy, List, ReadError, WriteError};
use netbuf::Buf;

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
    Ready(Easy, Arc<Mutex<Buf>>),
}


pub fn prepare(mut request: Request, hostport: String,
               settings: Arc<Proxy>, resp_buf: Arc<Mutex<Buf>>)
    -> Easy
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
    if request.body.is_some() {
        let mut body = request.body.take().unwrap();
        let clen = body.data.len();
        headers.append(format!("Content-Length: {}", clen).as_str());

        easy.read_function(move |mut buf| {
            println!("Writing data: {}", body.data.len());
            match buf.write(&body.data[..]) {
                Ok(bytes) => {
                    body.data.consume(bytes);
                    Ok(bytes)
                }
                // XXX: need to handle io::Error properly here;
                Err(e) => {
                    panic!("Write request body error: {:?}", e);
                    Err(ReadError::Abort)
                }
            }
        }).unwrap();
    }
    easy.http_headers(headers);

    easy.write_function(move |buf| {
        resp_buf.lock().unwrap()
        .write(buf)
        .map_err(|e| {
            panic!("Write response body error: {:?}", e);
            WriteError::Pause
        })
    });
    easy
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


pub fn serve<S>(mut response: Pickler<S>, mut resp: Easy, body: Arc<Mutex<Buf>>)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static,
{
    let buf = body.lock().unwrap().split_off(0);

    let code = resp.response_code().unwrap();
    // TODO: handle response codes respectively,
    //      ie 204 has no body.
    let status = Status::from(code as u16).unwrap();
    response.status(status);
    if status.response_has_body() {
        response.add_length(buf.len() as u64);
    }
    if response.done_headers() {
        if status.response_has_body() {
            response.write_body(&buf[..]);
        }
    };
    response.done().boxed()
}
