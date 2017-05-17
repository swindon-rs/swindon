use std::str::from_utf8;
use std::sync::Arc;
use std::io::BufWriter;
use std::mem;

use futures::Async;
use futures::future::{FutureResult, ok};
use tk_http::{Status, Version};
use tk_http::client as http;
use rustc_serialize::Encodable;
use rustc_serialize::json::{as_json, Json};

use chat::authorize::{parse_userinfo, good_status};
use chat::{Cid, ConnectionSender, ConnectionMessage, TangleAuth};
use chat::cid::{serialize_cid};
use chat::CloseReason::{AuthHttp};
use chat::ConnectionMessage::{Hello, StopSocket};
use chat::error::MessageError;
use chat::message::{AuthData, Auth, Call, Meta, Args, Kwargs};
use chat::processor::{ProcessorPool, Action};
use config::http_destinations::Destination;
use intern::SessionId;
use proxy::{Response};


const INACTIVITY_PAYLOAD: &'static [u8] = b"[{}, [], {}]";


enum AuthState {
    Init(String, AuthData),
    Wait,
    Headers(Status),
    Done(Response),
    Void,
}

enum CallState {
    Init {
        auth: Arc<String>,
        path: String, args: Args, kw: Kwargs
    },
    Wait,
    Headers(Status),
    Void,
}

pub struct AuthCodec {
    state: AuthState,
    chat: ProcessorPool,
    conn_id: Cid,
    destination: Arc<Destination>,
    sender: ConnectionSender,
}

pub struct CallCodec {
    state: CallState,
    meta: Arc<Meta>,
    conn_id: Cid,
    destination: Arc<Destination>,
    sender: ConnectionSender,
}

pub struct InactivityCodec {
    path: Arc<String>,
    destination: Arc<Destination>,
    session_id: SessionId,
}

impl AuthCodec {
    pub fn new(path: String, cid: Cid, req: AuthData,
        chat: ProcessorPool, destination: &Arc<Destination>,
        tx: ConnectionSender)
        -> AuthCodec
    {
        AuthCodec {
            state: AuthState::Init(path, req),
            chat: chat,
            conn_id: cid,
            destination: destination.clone(),
            sender: tx,
        }
    }
}

impl CallCodec {
    pub fn new(auth: Arc<String>, path: String, cid: Cid,
        meta: &Arc<Meta>, args: Args, kw: Kwargs,
        destination: &Arc<Destination>,
        sender: ConnectionSender)
        -> CallCodec
    {
        CallCodec {
            state: CallState::Init {
                auth: auth,
                path: path,
                args: args,
                kw: kw,
            },
            meta: meta.clone(),
            conn_id: cid,
            destination: destination.clone(),
            sender: sender,
        }
    }
}

impl InactivityCodec {
    pub fn new(path: &Arc<String>, sid: &SessionId,
        destination: &Arc<Destination>)
        -> InactivityCodec
    {
        InactivityCodec {
            path: path.clone(),
            destination: destination.clone(),
            session_id: sid.clone(),
        }
    }
}


fn write_json_request<S, E>(mut e: http::Encoder<S>, data: &E)
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


impl<S> http::Codec<S> for AuthCodec {
    type Future = FutureResult<http::EncoderDone<S>, http::Error>;

    fn start_write(&mut self, mut e: http::Encoder<S>) -> Self::Future {
        use self::AuthState::*;
        if let Init(p, i) = mem::replace(&mut self.state, Void)
        {
            self.state = Wait;
            e.request_line("POST", &p, Version::Http11);
            if let Some(ref header) = self.destination.override_host_header {
                e.add_header("Host", header).unwrap();
            }
            ok(write_json_request(e,
                &Auth(&serialize_cid(&self.conn_id), &i)))
        } else {
            panic!("wrong state");
        }
    }
    fn headers_received(&mut self, headers: &http::Head)
        -> Result<http::RecvMode, http::Error>
    {
        use self::AuthState::*;
        if let Wait = mem::replace(&mut self.state, Void) {
            self.state = Headers(
                headers.status().unwrap_or(Status::InternalServerError));
            // TODO(tailhook) limit and streaming
            Ok(http::RecvMode::buffered(10_485_760))
        } else {
            panic!("wrong state");
        }
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, http::Error>
    {
        use self::AuthState::*;
        // TODO(tailhook) streaming
        assert!(end);
        match mem::replace(&mut self.state, Void) {
            Headers(Status::Ok) => {
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
                        self.sender.send(Hello(sess_id.clone(),
                                               userinfo.clone()));
                        self.chat.send(Action::Associate {
                            conn_id: self.conn_id,
                            session_id: sess_id,
                            metadata: userinfo,
                        });
                    }
                    Err(()) => {
                        debug!("Auth error");
                        self.sender.send(StopSocket(
                            AuthHttp(Status::InternalServerError)));
                    }
                };
            }
            Headers(status) => {
                if good_status(status) {
                    self.sender.send(StopSocket(AuthHttp(status)));
                } else {
                    self.sender.send(StopSocket(
                        AuthHttp(Status::InternalServerError)));
                }
            }
            _ => unreachable!(),
        }
        Ok((Async::Ready(data.len())))
    }
}

impl<S> http::Codec<S> for CallCodec {
    type Future = FutureResult<http::EncoderDone<S>, http::Error>;

    fn start_write(&mut self, mut e: http::Encoder<S>) -> Self::Future {
        use self::CallState::*;
        if
            let Init { auth, path, args, kw} =
            mem::replace(&mut self.state, Void)
        {
            e.request_line("POST", &path, Version::Http11);
            if let Some(ref header) = self.destination.override_host_header {
                e.add_header("Host", header).unwrap();
            }
            // TODO(tailhook) implement authrization
            e.add_header("Authorization", &*auth).unwrap();
            let cid = serialize_cid(&self.conn_id);
            let done = write_json_request(e, &Call(&*self.meta, &cid, &args, &kw));
            self.state = Wait;
            ok(done)
        } else {
            panic!("wrong state");
        }
    }
    fn headers_received(&mut self, headers: &http::Head)
        -> Result<http::RecvMode, http::Error>
    {
        use self::CallState::*;
        if let Wait = mem::replace(&mut self.state, Void) {
            self.state = Headers(
                headers.status().unwrap_or(Status::InternalServerError));
            // TODO(tailhook) configure limit
            Ok(http::RecvMode::buffered(10_485_760))
        } else {
            panic!("wrong state");
        }
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, http::Error>
    {
        use self::CallState::*;
        assert!(end);
        match mem::replace(&mut self.state, Void) {
            Headers(Status::Ok) => {
                // TODO: connection_id must be dropped from meta
                match parse_response(data) {
                    Ok(x) => {
                        self.sender.send(ConnectionMessage::Result(
                            self.meta.clone(), x));
                    }
                    Err(e) => {
                        self.sender.send(ConnectionMessage::Error(
                            self.meta.clone(), e));
                    }
                }
            }
            Headers(status) => {
                self.sender.send(ConnectionMessage::Error(self.meta.clone(),
                    // TODO(tailhook) should we put body here?
                    MessageError::HttpError(status, None)));
            }
            _ => unreachable!(),
        }
        Ok((Async::Ready(data.len())))
    }
}

impl<S> http::Codec<S> for InactivityCodec {
    type Future = FutureResult<http::EncoderDone<S>, http::Error>;

    fn start_write(&mut self, mut e: http::Encoder<S>) -> Self::Future {
        e.request_line("POST", &self.path, Version::Http11);
        if let Some(ref header) = self.destination.override_host_header {
            e.add_header("Host", header).unwrap();
        }
        // TODO(tailhook) implement authrization
        e.format_header("Authorization",
                        TangleAuth(&self.session_id)).unwrap();
        e.add_header("Content-Type", "application/json").unwrap();
        e.add_length(INACTIVITY_PAYLOAD.len() as u64).unwrap();
        e.done_headers().unwrap();
        e.write_body(INACTIVITY_PAYLOAD);
        ok(e.done())
    }
    fn headers_received(&mut self, _: &http::Head)
        -> Result<http::RecvMode, http::Error>
    {
        // TODO(tailhook) retry request if failed
        Ok(http::RecvMode::buffered(0))
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, http::Error>
    {
        assert!(end);
        Ok((Async::Ready(data.len())))
    }
}

fn parse_response(data: &[u8]) -> Result<Json, MessageError> {
    Ok(Json::from_str(from_utf8(data)?)?)
}
