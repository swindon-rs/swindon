use std::sync::Arc;

use futures::AsyncSink;
use futures::future::{FutureResult, ok, err};
use futures::sink::{Sink};
use minihttp::websocket;
use minihttp::websocket::{Error};
use minihttp::websocket::Frame::{self, Text, Binary, Ping, Pong};
use tokio_core::reactor::Handle;
use rustc_serialize::json::Json;

use runtime::Runtime;
use config::chat::Chat;
use config::SessionPool;
use chat::{Cid, ConnectionSender};
use chat::cid::serialize_cid;
use chat::message::{decode_message, get_active, Meta, Args, Kwargs};
use chat::processor::{ProcessorPool, ConnectionMessage};
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

impl websocket::Dispatcher for Dispatcher {
    type Future = FutureResult<(), Error>;
    fn frame(&mut self, frame: &Frame) -> FutureResult<(), Error> {
        match *frame {
            Text(data) => match decode_message(data) {
                Ok((name, mut meta, args, kwargs)) => {
                    if let Some(duration) = get_active(&meta) {
                        // TODO(tailhook) update activity
                    }
                    meta.insert("connection_id".to_string(),
                        Json::String(serialize_cid(&self.cid)));
                    self.method_call(name, meta, args, kwargs);
                    ok(()) // no backpressure, yet
                }
                Err(e) => {
                    debug!("Message error: {}", e);
                    // TODO(tailhook) better error
                    err(Error::Closed)
                }
            },
            Binary(_) => {
                debug!("Binary messages are not supported yet");
                // TODO(tailhook) better error
                err(Error::Closed)
            }
            Ping(_)|Pong(_) => unreachable!(),
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
            path, &meta, args, kw,
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
}
