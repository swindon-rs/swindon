use std::sync::Arc;
use std::time::{Instant, Duration};
use std::cmp;

use futures::AsyncSink;
use futures::future::{FutureResult, ok, err};
use futures::sink::{Sink};
use minihttp::websocket;
use minihttp::websocket::{Error as WsError};
use minihttp::websocket::Frame::{self, Text, Binary, Ping, Pong, Close};
use tokio_core::reactor::Handle;
use rustc_serialize::json::Json;

use runtime::Runtime;
use config::chat::Chat;
use config::SessionPool;
use chat::{Cid, ConnectionSender, CloseReason};
use chat::cid::serialize_cid;
use chat::message::{decode_message, get_active, Meta, Args, Kwargs};
use chat::message::{ValidationError};
use chat::processor::{Action, ProcessorPool, ConnectionMessage};
use chat::backend::CallCodec;
use chat::error::MessageError;


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
        Validation(e: ValidationError) {
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
        match *frame {
            Text(data) => match decode_message(data) {
                Ok((name, meta, args, kwargs)) => {
                    if let Some(duration) = get_active(&meta) {
                        self.update_activity(duration);
                    }
                    self.method_call(name, meta, args, kwargs);
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
        let dest = self.settings.message_handlers.resolve(&name);
        let mut path = name.replace(".", "/");
        if dest.path == "/" {
            path.insert(0, '/');
        } else {
            path = dest.path.clone() + "/" + &path;
        };
        let mut up = self.runtime.http_pools.upstream(&dest.upstream);
        let meta = Arc::new(meta);
        let codec = Box::new(CallCodec::new(
            self.auth.clone(),
            path, self.cid, &meta, args, kw,
            self.channel.clone()));
        match up.get_mut().get_mut() {
            Some(pool) => {
                match pool.start_send(codec) {
                    Ok(AsyncSink::NotReady(codec)) => {
                        self.channel.send(ConnectionMessage::Error(meta,
                            MessageError::PoolOverflow));
                    }
                    Ok(AsyncSink::Ready) => {
                        debug!("Sent /tangle/authorize_connection to proxy");
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
