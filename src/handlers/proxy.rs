use std::str;
use std::sync::{Arc, Mutex};
use std::io::Write;
use std::ascii::AsciiExt;

use futures::{BoxFuture, Future};
use tokio_core::io::Io;
use minihttp::{Error, Request, Status};
use minihttp::enums::{Method, Header};
use tk_bufstream::IoBuf;
use curl::easy::{Easy, List};   //, ReadError, WriteError};
use curl::Error as CurlError;
use netbuf::Buf;
use httparse;

use config::proxy::Proxy;
use {Pickler};


pub enum ProxyCall {
    Prepare {
        hostport: String,
        settings: Arc<Proxy>,
    },
    Ready(Easy, usize, Buf),
}


/// Build and return curl.Easy handle.
///
/// Response body will be written into `resp_bu``.
/// `headers_counter` will hold number of response headers parsed by curl.
pub fn prepare(mut request: Request, hostport: String,
               settings: Arc<Proxy>,
               resp_buf: Arc<Mutex<Buf>>,
               headers_counter: Arc<Mutex<usize>>)
    -> Result<Easy, CurlError>
{
    let mut curl = Easy::new();
    let mut headers = List::new();

    // NOTE: close connections because of curl bug;
    curl.forbid_reuse(true).unwrap();
    try!(headers.append("Connection: close"));

    try!(curl.url(format!("http://{}{}", hostport, request.path).as_str()));

    match request.method {
        // TODO: implement all methods
        Method::Get => {
            try!(curl.get(true))
        }
        Method::Post => {
            try!(curl.post(true))
        }
        Method::Head => {
            try!(curl.nobody(true));
            try!(curl.custom_request("HEAD"));
        }
        Method::Patch => {
            try!(curl.custom_request("PATCH"));
        }
        Method::Put => {
            try!(curl.custom_request("PUT"));
        }
        Method::Other(m) => {
            try!(curl.custom_request(m.as_str()));
        }
        _ => panic!("Not implemented"),
    };

    let ip_addr = request.peer_addr.ip();
    try!(headers.append(
        format!("{}: {}", settings.ip_header, ip_addr)
        .as_str()));

    // Copy headers
    for &(ref name, ref value) in &request.headers {
        // TODO: validate headers
        let line = match name {
            &Header::Host => format!("Host: {}", value),
            &Header::Raw(ref name) => format!("{}: {}", name, value),
            _ => continue,
        };
        try!(headers.append(line.as_str()));
    }
    if request.body.is_some() {
        let mut body = request.body.take().unwrap();
        let clen = body.data.len();

        try!(headers.append(format!("Content-Length: {}", clen).as_str()));

        try!(curl.read_function(move |mut buf| {
            match buf.write(&body.data[..]) {
                Ok(bytes) => {
                    body.data.consume(bytes);
                    Ok(bytes)
                }
                // XXX: need to handle io::Error properly here;
                Err(e) => {
                    panic!("Write request body error: {:?}", e);
                    // Err(ReadError::Abort)
                }
            }
        }));
    }
    // TODO: setup response headers collect function;
    let headers_buf = resp_buf.clone();
    try!(curl.header_function(move |line| {
        if line.starts_with(b"HTTP/1.") {
            true
        } else {
            if !line.starts_with(b"\r\n") {
                *headers_counter.lock().unwrap() += 1;
            }
            headers_buf.lock().unwrap()
            .write(line)
            .map(|_| true)
            .unwrap_or(false)
        }
    }));

    // Setup response collect function;
    try!(curl.write_function(move |buf| {
        resp_buf.lock().unwrap()
        .write(buf)
        .map_err(|e| {
            panic!("Write response body error: {:?}", e);
            // WriteError::Pause
        })
    }));

    try!(curl.http_headers(headers));
    Ok(curl)
}


pub fn serialize<S>(mut response: Pickler<S>, mut resp: Easy,
        num_headers: usize, body: Buf)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static,
{
    let mut headers = vec![httparse::EMPTY_HEADER; num_headers];
    let body_offset = match httparse::parse_headers(&body[..], &mut headers) {
        Ok(httparse::Status::Complete((bytes, _))) => bytes,
        _ => {
            // TODO: write ErrorRepsonse
            unreachable!();
        }
    };

    let code = resp.response_code().unwrap();
    // TODO: handle response codes respectively,
    //      ie 204 has no body.
    let status = Status::from(code as u16).unwrap();
    response.status(status);

    for h in headers.iter() {
        if h.name.eq_ignore_ascii_case("Content-Length") {
            response.add_length((body.len() - body_offset) as u64);
        } else if h.name.eq_ignore_ascii_case("Transfer-encoding") {
            response.add_chunked();
        } else {
            response.add_header(h.name, h.value);
        }
    }

    if response.done_headers() {
        if status.response_has_body() {
            response.write_body(&body[body_offset..]);
        }
    };
    response.done().boxed()
}
