use std::str;
use std::ascii::AsciiExt;
use tk_http::websocket::client::{self as ws, Head, Encoder, EncoderDone};
use tk_http::websocket::Error;

use runtime::ServerId;


pub struct Authorizer {
    server_id: ServerId,
    peername: String,
}


impl Authorizer {
    pub fn new(peer: String, server_id: ServerId)
        -> Authorizer
    {
        Authorizer {
            server_id: server_id,
            peername: peer,
        }
    }
}

impl<S> ws::Authorizer<S> for Authorizer {
    type Result = ServerId;

    fn write_headers(&mut self, mut e: Encoder<S>)
        -> EncoderDone<S>
    {
        e.request_line("/v1/swindon-chat");
        e.format_header("Host", &self.peername).unwrap();
        e.format_header("Origin",
            format_args!("http://{}/v1/swindon-chat", self.peername)).unwrap();
        e.format_header("X-Swindon-Node-Id", &self.server_id).unwrap();
        e.done()
    }

    fn headers_received(&mut self, headers: &Head)
        -> Result<Self::Result, Error>
    {
        headers.all_headers().iter()
        .find(|h| h.name.eq_ignore_ascii_case("X-Swindon-Node-Id"))
        .ok_or(Error::custom("missing X-Swindon-Node-Id header"))
        .and_then(|h| str::from_utf8(h.value)
            .map_err(|_| Error::custom("invalid node id")))
        .and_then(|s| s.parse()
            .map_err(|_| Error::custom("invalid node id")))
    }
}
