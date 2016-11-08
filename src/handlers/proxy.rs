use std::str;
use std::sync::Arc;
use std::convert::From;

use futures::{BoxFuture, Future};
use tokio_core::io::Io;
use minihttp::{Error, Request};
use minihttp::enums::Header;
use minihttp::client::{Response, HttpClient};
use tk_bufstream::IoBuf;

use config::proxy::Proxy;
use {Pickler};


pub enum ProxyCall {
    Prepare {
        hostport: String,
        settings: Arc<Proxy>,
    },
    Ready {
        response: Response,
    },
}


/// Build and return curl.Easy handle.
///
/// Response body will be written into `resp_bu``.
/// `headers_counter` will hold number of response headers parsed by curl.
pub fn prepare(mut request: Request, hostport: String,
               settings: Arc<Proxy>,
               client: &mut HttpClient)
{
    client.request(
        request.method,
        format!("http://{}{}", hostport, request.path).as_str());

    let ip_addr = format!("{}", request.peer_addr.ip());
    client.add_header(
        Header::from(settings.ip_header.as_str()), ip_addr.as_str());

    // Copy headers
    for &(ref name, ref value) in &request.headers {
        // TODO: validate headers
        match name {
            &Header::Host => client.add_header(Header::Host, value),
            &Header::Raw(ref name) => client.add_header(    
                Header::from(name.as_str()), value),
            _ => continue,
        };
    }
    if request.body.is_some() {
        let body = request.body.take().unwrap();
        let clen = body.data.len();
        client.add_length(clen as u64);
        client.done_headers();
        client.body_from_buf(body.data);
    } else {
        client.done_headers();
    }
}


/// Serialize buffered response.
///
pub fn serialize<S>(mut response: Pickler<S>, resp: Response)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static,
{
    // TODO: handle response codes respectively,
    //      ie 204 has no body.
    response.status(resp.status.clone());

    if resp.body.is_some() {
        if let Some(len) = resp.content_length() {
            response.add_length(len);
        }
    }
    for (ref header, ref value) in resp.headers {
        match header {
            &Header::TransferEncoding => {
                println!("Adding Chunked response");
                response.add_chunked();
            }
            &Header::Raw(ref name) => {
                response.add_header(name.as_str(), value);
            }
            _ => {} // ignore
        }
    }

    if response.done_headers() {
        if resp.status.response_has_body() {
            if let Some(ref body) = resp.body {
                response.write_body(&body[..]);
            }
        }
    };
    response.done().boxed()
}
