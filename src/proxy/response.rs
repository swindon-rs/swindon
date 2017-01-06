use tokio_core::io::Io;

use minihttp::{Status};
use minihttp::client::Head;
use minihttp::server::{EncoderDone};

use incoming::Encoder;


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
        match self.status {
            RespStatus::Normal(s) => e.status(s),
            RespStatus::Custom(c, ref r) => e.custom_status(c, r),
        }
        for &(ref k, ref v) in &self.headers {
            e.add_header(k, v);
        }
        e.add_length(self.body.len() as u64);
        if e.done_headers() {
            e.write_body(&self.body);
        }
        return e.done();
    }
}
