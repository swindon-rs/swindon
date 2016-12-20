//! Chat API implementation.
use std::io;
use std::cmp;
use std::str::{self, FromStr};
use std::sync::Arc;
use std::borrow::Cow;
use std::time::{Instant, Duration};

use futures::{Future, BoxFuture, done};
use futures::sync::mpsc::{UnboundedSender as Sender};
use tokio_core::reactor::Handle;
use minihttp::server::{Request};
use minihttp::{OptFuture};
use minihttp::enums::{Status, Method, Version};
use minihttp::client::{Error};
use rustc_serialize::json::{self, Json};
use rand::thread_rng;

use intern::SessionId;
use websocket::Base64;
use config::{Config, SessionPool, InactivityTimeouts};
use config::chat::Chat;
use super::{Cid, ProcessorPool};
use super::{serialize_cid};
use super::processor::{Action, ConnectionMessage, PoolMessage};
use super::message::{self, Meta, Args, Kwargs};
use super::error::MessageError;
use http_pools::HttpPools;
use json_requests::request_fn_buffered;

// TODO: make ChatAPI Pool
//  bound to session pool config

const INACTIVITY_PAYLOAD: &'static [u8] = b"[{}, [], {}]";

pub struct ChatAPI {
    // shared between connections
    client: HttpPools,
    chat_config: Arc<Chat>,      // singleton per handler endpoint;
    proc_pool: ProcessorPool,   // singleton per handler endpoint;
    inactivity_timeouts: Arc<InactivityTimeouts>,
}


pub struct SessionAPI {
    api: ChatAPI,

    session_id: SessionId,
    auth_token: String,
    conn_id: Cid,   // connection id should not be used here;
    channel: Sender<ConnectionMessage>,
}

impl ChatAPI {

    pub fn new(http_client: HttpPools, chat: Arc<Chat>,
        proc_pool: ProcessorPool, inactivity_timeouts: Arc<InactivityTimeouts>)
        -> ChatAPI
    {
        ChatAPI {
            client: http_client,
            chat_config: chat,
            proc_pool: proc_pool,
            inactivity_timeouts: inactivity_timeouts,
        }
    }

    /// Issue Auth call to backend.
    ///
    /// Send Auth message to proper backend
    /// returninng Hello/Error message.
    pub fn authorize_connection(&mut self, req: &Request, conn_id: Cid,
        channel: Sender<ConnectionMessage>)
        -> OptFuture<Json, Error>
    {
        let http_cookies = req.headers.iter()
            .filter(|&&(ref k, _)| k == "Cookie")
            .map(|&(_, ref v)| v.clone())
            .collect::<String>();
        let http_auth = req.headers.iter()
            .find(|&&(ref k, _)| k == "Authorization")
            .map(|&(_, ref v)| v.clone());
        let url_qs = req.path.splitn(2, "?").nth(1).unwrap_or("").to_string();

        let data = message::AuthData {
            http_cookie: Some(http_cookies),
            http_authorization: http_auth,
            url_querystring: url_qs,
        };

        let payload = message::encode_auth(&serialize_cid(&conn_id), &data);

        self.proc_pool.send(Action::NewConnection {
            conn_id: conn_id,
            channel: channel,
        });

        let dest = self.chat_config.message_handlers
            .resolve("tangle.authorize_connection");
        let path: Cow<_> = if dest.path == "/" {
            "/tangle/authorize_connection".into()
        } else {
            (dest.path.to_string() + "/tangle/authorize_connection").into()
        };
        request_fn_buffered(self.client.upstream(&dest.upstream),
            move |mut e| {
                e.request_line("POST", &path, Version::Http11);
                e.add_header("Content-Type", "application/json").unwrap();
                e.add_length(payload.as_bytes().len() as u64).unwrap();
                e.done_headers().unwrap();
                e.write_body(payload.as_bytes());
                e.done().into()
            })
    }

    /// Make instance of Session API (api bound to cid/ssid/tx-channel)
    /// and associate this session with ws connection
    /// (send `Action::Associate`)
    pub fn session_api(self, session_id: SessionId, conn_id: Cid,
        userinfo: Json, mut channel: Sender<ConnectionMessage>)
        -> SessionAPI
    {

        let userinfo = Arc::new(userinfo);
        channel.send(ConnectionMessage::Hello(userinfo.clone()))
        .expect("message sent");

        self.proc_pool.send(Action::Associate {
            conn_id: conn_id,
            session_id: session_id.clone(),
            metadata: userinfo,
        });

        SessionAPI {
            api: self,
            auth_token: encode_sid(&session_id),
            session_id: session_id,
            conn_id: conn_id,
            channel: channel,
        }
    }

    /// API call to backend.
    fn post(&mut self, method: &str, auth: String, payload: String)
        -> OptFuture<Json, MessageError>
    {
        let dest = self.chat_config.message_handlers.resolve(method);
        // TODO(tailhook) replace here can be optimized
        let path = format!("{}{}", dest.path, method.replace(".", "/"));
        request_fn_buffered(self.client.upstream(&dest.upstream),
            move |mut e| {
                e.request_line("POST", &path, Version::Http11);
                e.add_header("Content-Type", "application/json").unwrap();
                e.add_header("Authorization", auth);
                e.add_length(payload.as_bytes().len() as u64).unwrap();
                e.done_headers().unwrap();
                e.write_body(payload.as_bytes());
                e.done().into()
            })
    }

    /// Update session activity timeout.
    fn update_activity(&self, conn_id: Cid, seconds: Duration) {
        let timestamp = Instant::now() + seconds;
        self.proc_pool.send(Action::UpdateActivity {
            conn_id: conn_id,
            timestamp: timestamp,
        })
    }
}

// only difference from ChatAPI -> Bound to concrete SessionId
impl SessionAPI {

    /// Send disconnect to processor.
    pub fn disconnect(&self) {
        self.api.proc_pool.send(Action::Disconnect { conn_id: self.conn_id });
    }

    /// 'Session active' notification for chat processor.
    pub fn update_activity(&self, sec: Option<u64>) {
        let normalized = match sec {
            Some(v) => {
                let v = Duration::from_secs(v);
                let min = *self.api.inactivity_timeouts.client_min;
                let max = *self.api.inactivity_timeouts.client_max;
                cmp::max(cmp::min(v, max), min)
            }
            None => {
                *self.api.inactivity_timeouts.client_default
            }
        };
        self.api.update_activity(self.conn_id.clone(), normalized)
    }

    /// Backend method call.
    pub fn method_call(&mut self, method: String, mut meta: Meta,
        args: &Args, kwargs: &Kwargs, handle: &Handle)
    {
        let mut tx = self.channel.clone();
        meta.insert("connection_id".to_string(),
            Json::String(serialize_cid(&self.conn_id)));
        let payload = message::encode_call(&meta, &args, &kwargs);
        let call = self.api.post(method.as_str(),
            // TODO(tailhook) optimize this clone?
            self.auth_token.clone(), payload);
        handle.spawn(call
            .then(move |result| {
                meta.remove(&"connection_id".to_string());
                let res = match result {
                    Ok(json) => tx.send(ConnectionMessage::Result(meta, json)),
                    Err(err) => {
                        err.update_meta(&mut meta);
                        tx.send(ConnectionMessage::Error(meta, err))
                    }
                };
                res.map_err(|e| info!("Remote send error: {:?}", e))
            })
        );
    }
}


/// Parse userinfo received on Auth call;
pub fn parse_userinfo(response: Json)
    -> Result<(SessionId, Json), MessageError>
{
    use super::message::ValidationError::*;
    use super::error::MessageError::*;
    match response {
        Json::Object(data) => {
            let sess_id = match data.get("user_id".into()) {
                Some(&Json::String(ref s)) => {
                    SessionId::from_str(s.as_str())
                    .map_err(|_| ValidationError(InvalidUserId))?
                }
                _ => return Err(ValidationError(InvalidUserId)),
            };
            Ok((sess_id, Json::Object(data)))
        }
        _ => {
            Err(ValidationError(ObjectExpected))
        }
    }
}

pub struct MaintenanceAPI {
    config: Arc<Config>,
    sessions_cfg: Arc<SessionPool>,
    http_client: HttpPools,
    handle: Handle,
}

impl MaintenanceAPI {

    pub fn new(cfg: Arc<Config>, sessions_cfg: Arc<SessionPool>,
        http_client: HttpPools, handle: Handle)
        -> MaintenanceAPI
    {
        MaintenanceAPI {
            config: cfg,
            sessions_cfg: sessions_cfg,
            http_client: http_client,
            handle: handle,
        }
    }

    pub fn handle(&self, message: PoolMessage) {
        use super::processor::PoolMessage::*;
        match message {
            InactiveSession { session_id, .. } => {
                info!("Send inactivity: {:?}", session_id);

                let auth = encode_sid(&session_id);

                for dest in &self.sessions_cfg.inactivity_handlers {
                    let path: Cow<_> = if dest.path == "/" {
                        "/tangle/session_inactive".into()
                    } else {
                        (dest.path.to_string() +
                            "/tangle/session_inactive").into()
                    };
                    // TODO(tailhook) optimize this auth.clone()
                    let auth = auth.clone();
                    self.handle.spawn(request_fn_buffered(
                        self.http_client
                            .upstream(&dest.upstream),
                        move |mut e| {
                            e.request_line("POST", &path, Version::Http11);
                            e.add_header("Content-Type", "application/json")
                                .unwrap();
                            e.add_header("Authorization", auth).unwrap();
                            e.add_length(INACTIVITY_PAYLOAD.len() as u64)
                                .unwrap();
                            e.done_headers().unwrap();
                            e.write_body(INACTIVITY_PAYLOAD);
                            e.done()
                        })
                        .map(|r| info!("Resp data: {}", r))
                        .map_err(|e: MessageError| {
                            info!("Error sending inactivity {}", e)
                        })
                    );
                }
            }
        }
    }
}

fn encode_sid(s: &SessionId) -> String {
    #[derive(RustcEncodable)]
    struct Auth<'a> {
        user_id: &'a SessionId,
    }
    format!("Tangle {}", Base64(json::encode(&Auth {
        user_id: s,
    }).unwrap().as_bytes()))
}
