use tokio_core::io::Io;

use tk_http::{Status};
use tk_http::client::Head;
use tk_http::server::{EncoderDone};

use incoming::Encoder;


#[derive(Debug)]
pub enum RespStatus {
    Normal(Status),
    Custom(u16, String),
}


pub struct HalfResp {
    status: RespStatus,
    headers: Vec<(String, Vec<u8>)>,
}

pub struct Response {
    status: RespStatus,
    headers: Vec<(String, Vec<u8>)>,
    body: Vec<u8>,
}

impl HalfResp {
    pub fn from_headers(head: &Head) -> HalfResp {
        let status = head.status().map(RespStatus::Normal)
            .unwrap_or_else(|| {
                let (c, v) = head.raw_status();
                RespStatus::Custom(c, v.to_string())
            });
        HalfResp {
            status: status,
            headers: head.headers().map(|(k, v)| {
                (k.to_string(), v.to_vec())
            }).collect(),
        }
    }
    pub fn complete(self, body: Vec<u8>) -> Response {
        Response {
            status: self.status,
            headers: self.headers,
            body: body,
        }
    }
}

impl Response {
    pub fn encode<S:Io>(&self, mut e: Encoder<S>) -> EncoderDone<S>{
        let body = match self.status {
            RespStatus::Normal(s) => {
                e.status(s);
                s.response_has_body()
            }
            RespStatus::Custom(c, ref r) => {
                e.custom_status(c, r);
                true
            }
        };
        for &(ref k, ref v) in &self.headers {
            e.add_header(k, v);
        }
        if body {
            e.add_length(self.body.len() as u64);
            if e.done_headers() {
                e.write_body(&self.body);
            }
        } else {
            let res = e.done_headers();
            assert!(res == false);
        }
        return e.done();
    }
}
