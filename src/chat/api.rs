//! Chat API implementation.
use std::io;
use std::cmp;
use std::str::{self, FromStr};
use std::sync::Arc;
use std::time::{Instant, Duration};

use futures::{Future, BoxFuture, done};
use futures::sync::mpsc::{UnboundedSender as Sender};
use tokio_core::reactor::Handle;
use minihttp::Request;
use minihttp::enums::{Status, Method};
use minihttp::client::{HttpClient, Response as ClientResponse};
use rustc_serialize::json::{self, Json};
use rand::thread_rng;

use intern::SessionId;
use websocket::Base64;
use config::{Config, SessionPool};
use super::{Cid, ProcessorPool};
use super::{serialize_cid};
use super::router::{MessageRouter, url_for};
use super::processor::{Action, ConnectionMessage, PoolMessage};
use super::message::{self, Meta, Args, Kwargs};
use super::error::MessageError;

// min & max session activity seconds;
// TODO: get from config; store in ChatAPI
const MIN_SESSION_ACTIVE: u64 = 1;
const MAX_SESSION_ACTIVE: u64 = 30;

pub struct ChatAPI {
    // shared between connections
    client: HttpClient,         // gets cloned for each request;
    router: MessageRouter,      // singleton per handler endpoint;
    proc_pool: ProcessorPool,   // singleton per handler endpoint;
}

pub struct SessionAPI {
    api: ChatAPI,

    session_id: SessionId,
    auth_token: String,
    conn_id: Cid,   // connection id should not be used here;
    channel: Sender<ConnectionMessage>,
}

impl ChatAPI {
    pub fn new(http_client: HttpClient, router: MessageRouter,
        proc_pool: ProcessorPool)
        -> ChatAPI
    {
        ChatAPI {
            client: http_client,
            router: router,
            proc_pool: proc_pool,
        }
    }

    /// Issue Auth call to backend.
    ///
    /// Send Auth message to proper backend
    /// returninng Hello/Error message.
    pub fn authorize_connection(&self, req: &Request, conn_id: Cid,
        channel: Sender<ConnectionMessage>)
        -> BoxFuture<ClientResponse, io::Error>
    {
        let http_cookies = req.headers.iter()
            .filter(|&&(ref k, _)| k == "Cookie")
            .map(|&(_, ref v)| v.clone())
            .collect::<String>();
        let http_auth = req.headers.iter()
            .find(|&&(ref k, _)| k == "Authorization")
            .map(|&(_, ref v)| v.clone())
            .unwrap_or("".to_string());

        let mut data = Kwargs::new();
        // TODO: parse cookie string to hashmap;
        data.insert("http_cookie".into(),
            Json::String(http_cookies));
        data.insert("http_authorization".into(),
            Json::String(http_auth));

        let payload = message::encode_auth(&serialize_cid(&conn_id), &data);

        self.proc_pool.send(Action::NewConnection {
            conn_id: conn_id,
            channel: channel,
        });

        let mut req = self.client.clone();
        req.request(Method::Post,
            self.router.get_auth_url().as_str());
        req.add_header("Content-Type".into(), "application/json");
        req.add_length(payload.as_bytes().len() as u64);
        req.done_headers();
        req.write_body(payload.as_bytes());
        req.done()
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
    fn post(&self, method: &str, auth: &str, payload: &[u8])
        -> BoxFuture<Json, MessageError>
    {
        let url = self.router.resolve(method);
        let mut req = self.client.clone();
        req.request(Method::Post, url.as_str());
        req.add_header("Content-Type".into(), "application/json");
        req.add_header("Authorization".into(), auth);
        req.add_length(payload.len() as u64);
        req.done_headers();
        req.write_body(payload);
        req.done()
        .map_err(|e| e.into())
        .and_then(|resp| done(parse_response(resp)))
        .boxed()
    }

    /// Update session activity timeout.
    fn update_activity(&self, conn_id: Cid, seconds: u64) {
        let seconds = cmp::max(
            cmp::min(seconds, MAX_SESSION_ACTIVE),
            MIN_SESSION_ACTIVE);
        let timestamp = Instant::now() + Duration::from_secs(seconds);
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
    pub fn update_activity(&self, sec: u64) {
        self.api.update_activity(self.conn_id.clone(), sec)
    }

    /// Backend method call.
    pub fn method_call(&self, method: String, mut meta: Meta,
        args: &Args, kwargs: &Kwargs, handle: &Handle)
    {
        let mut tx = self.channel.clone();
        meta.insert("connection_id".to_string(),
            Json::String(serialize_cid(&self.conn_id)));
        let payload = message::encode_call(&meta, &args, &kwargs);
        let call = self.api.post(method.as_str(),
            self.auth_token.as_str(), payload.as_bytes());
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


/// Parse backend response.
fn parse_response(response: ClientResponse) -> Result<Json, MessageError>
{
    // TODO: check content-type
    let payload = match response.body {
        Some(ref data) => {
            str::from_utf8(&data[..])
            .map_err(|e| MessageError::from(e))
            .and_then(
                |s| Json::from_str(s).map_err(|e| MessageError::from(e))
            )?
        }
        None => Json::Null,
    };
    match (response.status, payload) {
        (Status::Ok, payload) => Ok(payload),
        (s, Json::Null) => Err(MessageError::HttpError(s, None)),
        (s, payload) => Err(MessageError::HttpError(s, Some(payload))),
    }
}

/// Parse userinfo received on Auth call;
pub fn parse_userinfo(response: ClientResponse)
    -> Result<(SessionId, Json), MessageError>
{
    use super::message::ValidationError::*;
    use super::error::MessageError::*;
    match parse_response(response) {
        Ok(Json::Object(data)) => {
            let sess_id = match data.get("user_id".into()) {
                Some(&Json::String(ref s)) => {
                    SessionId::from_str(s.as_str())
                    .map_err(|_| ValidationError(InvalidUserId))?
                }
                _ => return Err(ValidationError(InvalidUserId)),
            };
            Ok((sess_id, Json::Object(data)))
        }
        Ok(_) => {
            Err(ValidationError(ObjectExpected))
        }
        Err(err) => {
            Err(err)
        }
    }
}

pub struct MaintenanceAPI {
    config: Arc<Config>,
    sessions_cfg: Arc<SessionPool>,
    http_client: HttpClient,
    handle: Handle,
}

impl MaintenanceAPI {

    pub fn new(cfg: Arc<Config>, sessions_cfg: Arc<SessionPool>,
        http_client: HttpClient, handle: Handle)
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
                    if let Some(url) = url_for(
                        "tangle/session_inactive",
                        &dest, &self.config.http_destinations,
                        &mut thread_rng())
                    {
                        let mut req = self.http_client.clone();
                        req.request(Method::Post, url.as_str());
                        req.add_header("Content-Type".into(),
                            "application/json");
                        req.add_header("Authorization".into(), auth.as_str());
                        // TODO: add empty message
                        req.add_length(0);
                        req.done_headers();
                        self.handle.spawn(req.done()
                        .map(|r| info!("Resp status: {:?}", r.status))
                        .map_err(|e| info!("Error sending inactivity {:?}", e))
                        );
                        // TODO: better messages
                    } else {
                        info!("No url for {:?}", dest);
                    }
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
