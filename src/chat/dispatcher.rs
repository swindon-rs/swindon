use std::sync::Arc;
use std::time::{Instant, Duration};
use std::cmp;

use futures::AsyncSink;
use futures::future::{FutureResult, ok, err};
use futures::sink::{Sink};
use tk_http::websocket;
use tk_http::websocket::{Error as WsError};
use tk_http::websocket::Frame::{self, Text, Binary, Ping, Pong, Close};
use tokio_core::reactor::Handle;
use serde_json::{Error as JsonError};

use http_pools::{REQUESTS, FAILED_503};
use runtime::Runtime;
use config::chat::Chat;
use config::SessionPool;
use chat::{Cid, ConnectionSender, CloseReason, CONNECTIONS};
use chat::message::{self, Meta, Args, Kwargs};
use chat::processor::{Action, ProcessorPool, ConnectionMessage};
use chat::backend::CallCodec;
use chat::error::MessageError;

use metrics::{Counter};

lazy_static! {
    pub static ref FRAMES_RECEIVED: Counter = Counter::new();
}

pub struct Dispatcher {
    pub cid: Cid,
    pub auth: Arc<String>,
    pub runtime: Arc<Runtime>,
    pub settings: Arc<Chat>,
    pub pool_settings: Arc<SessionPool>,
    pub processor: ProcessorPool,
    pub handle: Handle, // Does it belong here?
    pub channel: ConnectionSender,
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Validation(e: JsonError) {
            description(e.description())
            cause(e)
            from()
        }
        Binary {
            description("binary messages are not supported yet")
        }
    }
}

impl websocket::Dispatcher for Dispatcher {
    type Future = FutureResult<(), WsError>;
    fn frame(&mut self, frame: &Frame) -> FutureResult<(), WsError> {
        FRAMES_RECEIVED.incr(1);
        match *frame {
            Text(data) => match message::decode_message(data) {
                Ok((method, meta, args, kwargs)) => {
                    self.method_call(method, meta, args, kwargs);
                    ok(()) // no backpressure, yet
                }
                Err(e) => {
                    debug!("Message error: {}", e);
                    err(WsError::custom(Error::from(e)))
                }
            },
            Binary(_) => {
                debug!("Binary messages are not supported yet");
                // TODO(tailhook) better error
                err(WsError::custom(Error::Binary))
            }
            Ping(_)|Pong(_) => unreachable!(),
            Close(code, text) => {
                debug!("Received close message [{}]{:?}", code, text);
                self.channel.send(ConnectionMessage::StopSocket(
                    CloseReason::PeerClose(code, text.into())));
                ok(())
            }
        }
    }
}

impl Dispatcher {
    fn method_call(&self, name: String, meta: Meta, args: Args, kw: Kwargs) {
        let meta = Arc::new(meta);
        if !message::valid_method(&name) {
            self.channel.send(ConnectionMessage::Error(meta,
                MessageError::ValidationError(
                    "invalid metod".to_string())));
            return;
        }
        if !message::valid_request_id(&meta) {
            self.channel.send(ConnectionMessage::Error(meta,
                MessageError::ValidationError(
                    "invalid request id".to_string())));
            return;
        }
        if let Some(duration) = message::get_active(&meta) {
            self.update_activity(duration);
        }
        let dest = self.settings.message_handlers.resolve(&name);
        let mut path = name.replace(".", "/");
        if dest.path == "/" {
            path.insert(0, '/');
        } else {
            path = dest.path.clone() + "/" + &path;
        };
        let mut up = self.runtime.http_pools.upstream(&dest.upstream);
        let cfg = self.runtime.config.get();
        let dest_settings = match cfg.http_destinations.get(&dest.upstream) {
            Some(h) => h,
            None => {
                error!("No such destination {:?}", dest.upstream);
                self.channel.send(ConnectionMessage::Error(meta,
                    MessageError::PoolError));
                return;
            }
        };
        let codec = Box::new(CallCodec::new(
            self.auth.clone(),
            path, self.cid, &meta, args, kw,
            dest_settings,
            self.channel.clone(),
            self.runtime.server_id.clone()));
        match up.get_mut().get_mut() {
            Some(pool) => {
                match pool.start_send(codec) {
                    Ok(AsyncSink::NotReady(_codec)) => {
                        FAILED_503.incr(1);
                        self.channel.send(ConnectionMessage::Error(meta,
                            MessageError::PoolOverflow));
                    }
                    Ok(AsyncSink::Ready) => {
                        REQUESTS.incr(1);
                        debug!("Sent {} to chat backend", name);
                    }
                    Err(e) => {
                        error!("Error sending to pool {:?}: {}",
                            dest.upstream, e);
                        self.channel.send(ConnectionMessage::Error(meta,
                            MessageError::PoolError));
                    }
                }
            }
            None => {
                error!("No such destination {:?}", dest.upstream);
                self.channel.send(ConnectionMessage::Error(meta,
                    MessageError::PoolError));
            }
        }
    }

    fn update_activity(&self, seconds: u64) {
        let min = *self.pool_settings.client_min_idle_timeout;
        let max = *self.pool_settings.client_max_idle_timeout;
        let seconds = Duration::from_secs(seconds);
        let seconds = cmp::max(cmp::min(seconds, max), min);
        let timestamp = Instant::now() + seconds;
        self.processor.send(Action::UpdateActivity{
            conn_id: self.cid,
            timestamp: timestamp,
        });
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        self.processor.send(Action::Disconnect { conn_id: self.cid });
        CONNECTIONS.decr(1)
    }
}
