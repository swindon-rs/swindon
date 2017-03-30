use std::sync::Arc;
use std::net::SocketAddr;

use tk_http::Version;
use tk_http::client::{Encoder, EncoderDone};

use incoming::{Input};
use config::proxy::Proxy;
use request_id::RequestId;


/// A repeatable (so fully-buffered) request structure
#[derive(Clone, Debug)]
pub struct RepReq(Arc<ReqData>);

pub struct HalfReq {
    settings: Arc<Proxy>,
    method: String,
    path: String,
    host: String,
    headers: Vec<(String, Vec<u8>)>,
    addr: SocketAddr,
    request_id: RequestId,
}

#[derive(Debug)]
struct ReqData {
    settings: Arc<Proxy>,
    method: String,
    path: String,
    host: String,
    headers: Vec<(String, Vec<u8>)>,
    addr: SocketAddr,
    request_id: RequestId,
    body: Vec<u8>,
}


impl HalfReq {
    pub fn from_input(inp: &Input, settings: &Arc<Proxy>) -> HalfReq {
        use tk_http::server::RequestTarget::*;
        let path = match *inp.headers.request_target() {
            Origin(x) => x.to_string(),
            Absolute { path, ..} => path.to_string(),
            Authority(..) => unreachable!(),
            Asterisk => String::from("*"),
        };

        HalfReq {
            settings: settings.clone(),
            method: inp.headers.method().to_string(),
            path: path,
            host: inp.headers.host().expect("host exists").to_string(),
            headers: inp.headers.headers().map(|(k, v)| {
                (k.to_string(), v.to_vec())
            }).collect(),
            addr: inp.addr,
            request_id: inp.request_id,
        }
    }
    pub fn upgrade(self, body: Vec<u8>) -> RepReq {
        RepReq(Arc::new(ReqData {
            settings: self.settings,
            method: self.method,
            path: self.path,
            host: self.host,
            headers: self.headers,
            addr: self.addr,
            request_id: self.request_id,
            body: body,
        }))
    }
}
impl RepReq {
    pub fn encode<S>(&self, mut e: Encoder<S>) -> EncoderDone<S>{
        let ref r = *self.0;
        if r.settings.destination.path == "/" {
            e.request_line(&r.method, &r.path, Version::Http11);
        } else {
            e.request_line(&r.method,
                &format!("{}{}", r.settings.destination.path, r.path),
                Version::Http11);
        }

        // Spec doesn't mandate, but recomments it to be first
        e.add_header("Host", &r.host).unwrap();
        if let Some(ref h) = r.settings.ip_header {
            // NOTE: this can duplicate header value.
            e.format_header(h, r.addr.ip()).unwrap();
        }
        if let Some(ref h) = r.settings.request_id_header {
            // NOTE: this can duplicate header value.
            e.format_header(h, r.request_id).unwrap();
        }

        for &(ref k, ref v) in &r.headers {
            e.add_header(k, v).unwrap();
        }
        e.add_length(r.body.len() as u64).unwrap();
        e.done_headers().unwrap();
        if r.body.len() != 0 {
            e.write_body(&r.body);
        }
        return e.done();
    }
}
