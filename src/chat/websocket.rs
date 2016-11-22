//! Chat protocol.
use std::io;

use futures::{Future, BoxFuture};
use futures::stream::Stream;
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use tokio_core::channel::Receiver;
use minihttp::enums::Status;
use minihttp::Error as HttpError;
use tk_bufstream::IoBuf;
use rustc_serialize::json;

use websocket as ws;
use super::message;
use super::processor::ConnectionMessage;
use super::api::SessionAPI;
use Pickler;

pub fn negotiate<S>(mut response: Pickler<S>, init: ws::Init, remote: Remote,
    session_api: SessionAPI, channel: Receiver<ConnectionMessage>)
    -> BoxFuture<IoBuf<S>, HttpError>
    where S: Io + Send + 'static
{
    response.status(Status::SwitchingProtocol);
    response.add_header("Upgrade", "websocket");
    response.add_header("Connection", "upgrade");
    response.format_header("Sec-WebSocket-Accept", init.base64());
    response.done_headers();
    response.steal_socket()
    .and_then(move |socket: IoBuf<S>| {
        remote.spawn(move |handle| {
            let channel = channel.map(|msg| {
                ws::OutFrame::Text(json::encode(&msg).unwrap())
            });
            let dispatcher = ChatDispatcher(session_api, handle.clone());
            ws::WebsockProto::new(socket, dispatcher, channel)
            .map_err(|e| info!("Websocket error: {}", e))
        });
        Err(io::Error::new(io::ErrorKind::BrokenPipe,
                           "Connection is stolen for websocket"))
    })
    .map_err(|e: io::Error| e.into())
    .boxed()
}

struct ChatDispatcher(SessionAPI, Handle);

impl ws::Dispatcher for ChatDispatcher {

    fn dispatch(&mut self, frame: ws::Frame,
        _replier: &mut ws::ImmediateReplier)
        -> Result<(), ws::Error>
    {
        if let ws::Frame::Text(data) = frame {
            let result = message::decode_message(data);
            if let Ok((method, meta, args, kwargs)) = result {
                if let Some(duration) = message::get_active(&meta) {
                    self.0.update_activity(duration)
                }
                self.0.method_call(method, meta, &args, &kwargs, &self.1);
            }
        };
        Ok(())
    }
}
