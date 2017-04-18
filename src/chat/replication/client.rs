use std::str;
use std::ascii::AsciiExt;
use std::net::SocketAddr;
use tk_http::websocket::client::{self as ws, Head, Encoder, EncoderDone};
use tk_http::websocket::Error;

use runtime::RuntimeId;


pub struct Authorizer {
    runtime_id: RuntimeId,
    addr: SocketAddr,
}


impl Authorizer {
    pub fn new(addr: SocketAddr, runtime_id: RuntimeId) -> Authorizer {
        Authorizer {
            runtime_id: runtime_id,
            addr: addr,
        }
    }
}

impl<S> ws::Authorizer<S> for Authorizer {
    type Result = (SocketAddr, RuntimeId);

    fn write_headers(&mut self, mut e: Encoder<S>)
        -> EncoderDone<S>
    {
        e.request_line("/v1/swindon-chat");
        e.format_header("Host", &self.addr).unwrap();
        e.format_header("Origin",
            format_args!("http://{}/v1/swindon-chat", self.addr)).unwrap();
        e.format_header("X-Swindon-Node-Id", &self.runtime_id).unwrap();
        e.done()
    }

    fn headers_received(&mut self, headers: &Head)
        -> Result<Self::Result, Error>
    {
        headers.all_headers().iter()
        .find(|h| h.name.eq_ignore_ascii_case("X-Swindon-Node-Id"))
        .and_then(|h| str::from_utf8(h.value).ok())
        .and_then(|s| RuntimeId::from_str(s))
        .ok_or(Error::custom("invalid node id"))
        .map(|x| (self.addr.clone(), x))
    }
}
