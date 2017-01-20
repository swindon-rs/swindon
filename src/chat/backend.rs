use std::str::from_utf8;
use std::sync::Arc;
use std::io::BufWriter;
use std::mem;

use futures::Async;
use futures::future::{FutureResult, ok};
use futures::sync::oneshot;
use minihttp::{Status, Version};
use minihttp::client as http;
use tokio_core::io::Io;
use rustc_serialize::Encodable;
use rustc_serialize::json::{as_json, Json};

use intern::SessionId;
use proxy::{RepReq, HalfResp, Response};
use chat::message::{AuthData, Auth};
use chat::Cid;
use chat::cid::{serialize_cid};
use chat::processor::{ProcessorPool, Action};
use chat::authorize::parse_userinfo;

enum State {
    Init(String, AuthData),
    Wait,
    Headers(Status),
    Done(Response),
    Void,
}


pub struct AuthCodec {
    state: State,
    chat: ProcessorPool,
    conn_id: Cid,
    sender: Option<oneshot::Sender<Result<Arc<Json>, Status>>>,
}

impl AuthCodec {
    pub fn new(path: String, cid: Cid, req: AuthData, chat: ProcessorPool,
        tx: oneshot::Sender<Result<Arc<Json>, Status>>)
        -> AuthCodec
    {
        AuthCodec {
            state: State::Init(path, req),
            chat: chat,
            conn_id: cid,
            sender: Some(tx),
        }
    }
}


fn write_json_request<S: Io, E>(mut e: http::Encoder<S>, data: &E)
    -> http::EncoderDone<S>
    where E: Encodable,
{
    use std::io::Write;
    e.add_header("Content-Type", "application/json").unwrap();
    e.add_chunked().unwrap();
    e.done_headers().unwrap();
    let mut buf = BufWriter::new(e);
    write!(&mut buf, "{}", as_json(data)).unwrap();
    match buf.into_inner() {
        Ok(x) => x.done(),
        Err(_) => unreachable!(),
    }
}


impl<S: Io> http::Codec<S> for AuthCodec {
    type Future = FutureResult<http::EncoderDone<S>, http::Error>;

    fn start_write(&mut self, mut e: http::Encoder<S>) -> Self::Future {
        if let State::Init(p, i) = mem::replace(&mut self.state, State::Void)
        {
            self.state = State::Wait;
            e.request_line("POST", &p, Version::Http11);
            ok(write_json_request(e,
                &Auth(&serialize_cid(&self.conn_id), &i)))
        } else {
            panic!("wrong state");
        }
    }
    fn headers_received(&mut self, headers: &http::Head)
        -> Result<http::RecvMode, http::Error>
    {
        if let State::Wait = mem::replace(&mut self.state, State::Void) {
            self.state = State::Headers(
                headers.status().unwrap_or(Status::InternalServerError));
            // TODO(tailhook) limit and streaming
            Ok(http::RecvMode::Buffered(10_485_760))
        } else {
            panic!("wrong state");
        }
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, http::Error>
    {
        // TODO(tailhook) streaming
        assert!(end);
        match mem::replace(&mut self.state, State::Void) {
            State::Headers(hr) => {
                let result = from_utf8(data)
                    .map_err(|e| debug!("Invalid utf-8 in auth data: {}", e))
                .and_then(|s| Json::from_str(s)
                    .map_err(|e| debug!("Invalid json in auth data: {}", e)))
                .and_then(|j| parse_userinfo(j)
                    .map_err(|e| debug!("Bad user info in auth data: {}", e)));
                match result {
                    Ok((sess_id, userinfo)) => {
                        let userinfo = Arc::new(userinfo);
                        debug!("Auth data received {:?}: {:?}",
                            sess_id, userinfo);
                        self.chat.send(Action::Associate {
                            conn_id: self.conn_id,
                            session_id: sess_id,
                            metadata: userinfo.clone(),
                        });
                        self.sender.take().expect("not responded yet")
                            .complete(Ok(userinfo))
                    }
                    Err(()) => {
                        debug!("Auth error");
                        self.sender.take().expect("not responded yet")
                            .complete(Err(Status::InternalServerError));
                    }
                };
            }
            _ => unreachable!(),
        }
        Ok((Async::Ready(data.len())))
    }
}
