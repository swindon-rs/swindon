use std::sync::Arc;

use futures::future::{FutureResult, ok, err};
use minihttp::websocket;
use minihttp::websocket::{Error};
use minihttp::websocket::Frame::{self, Text, Binary, Ping, Pong};
use tokio_core::reactor::Handle;

use runtime::Runtime;
use config::chat::Chat;
use config::SessionPool;
use chat::message::decode_message;
use chat::processor::ProcessorPool;


pub struct Dispatcher {
    pub runtime: Arc<Runtime>,
    pub settings: Arc<Chat>,
    pub pool_settings: Arc<SessionPool>,
    pub processor: ProcessorPool,
    pub handle: Handle, // Does it belong here?
}

impl websocket::Dispatcher for Dispatcher {
    type Future = FutureResult<(), Error>;
    fn frame(&mut self, frame: &Frame) -> FutureResult<(), Error> {
        match *frame {
            Text(data) => match decode_message(data) {
                Ok((name, meta, args, kwargs)) => {
                    // TODO(tailhook) update activity
                    // send method call
                    unimplemented!();
                    ok(())
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
