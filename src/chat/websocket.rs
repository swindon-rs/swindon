//! Chat protocol.
use std::io;
use std::time::Duration;

use futures::{Future, BoxFuture};
use futures::stream::Stream;
use futures::sync::mpsc::{UnboundedReceiver as Receiver};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use minihttp::Status;
use minihttp::server::Error as HttpError;
use tk_bufstream::IoBuf;
use rustc_serialize::json;

use websocket as ws;
use websocket::write::WriteExt;
use super::message;
use super::processor::ConnectionMessage;
use super::api::SessionAPI;
use Pickler;
use flush_and_wait::FlushAndWait;

pub fn negotiate<S>(mut response: Pickler<S>, init: ws::Init, handle: Handle,
    session_api: SessionAPI, channel: Receiver<ConnectionMessage>)
    -> Box<Future<Item=IoBuf<S>, Error=HttpError>>
    where S: Io + Send + 'static
{
    response.status(Status::SwitchingProtocol);
    response.add_header("Upgrade", "websocket");
    response.add_header("Connection", "upgrade");
    response.format_header("Sec-WebSocket-Accept", init.base64());
    response.done_headers();
    Box::new(
        response.steal_socket()
        .and_then(move |socket: IoBuf<S>| {
            let h2 = handle.clone();
            handle.spawn_fn(move || {
                let channel = channel.map(|msg| {
                    match msg {
                        ConnectionMessage::StopSocket(reason) => {
                            ws::OutFrame::Close(reason)
                        }
                        msg => {
                            ws::OutFrame::Text(json::encode(&msg).unwrap())
                        }
                    }
                });
                let dispatcher = ChatDispatcher(session_api, h2);
                ws::WebsockProto::new(socket, dispatcher, channel)
                .map_err(|e| info!("Websocket error: {}", e))
            });
            // Ensure that original http server thinks connection is not useful
            Err(io::Error::new(io::ErrorKind::BrokenPipe,
                               "Connection is stolen for websocket"))
        })
        .map_err(|e: io::Error| e.into()))
}

pub fn fail<S>(mut response: Pickler<S>, init: ws::Init, handle: Handle,
    reason: ws::CloseReason)
    -> Box<Future<Item=IoBuf<S>, Error=HttpError>>
    where S: Io + 'static
{
    response.status(Status::SwitchingProtocol);
    response.add_header("Upgrade", "websocket");
    response.add_header("Connection", "upgrade");
    response.format_header("Sec-WebSocket-Accept", init.base64());
    response.done_headers();
    Box::new(response.steal_socket()
        .and_then(move |mut socket| {
            socket.out_buf.write_close(reason.code(), reason.reason());
            handle.spawn(
                FlushAndWait::new(socket, &handle, Duration::new(1, 0))
            );

            // Ensure that original http server thinks connection is not useful
            Err(io::Error::new(io::ErrorKind::BrokenPipe,
                               "Connection is stolen for websocket"))
        })
        .map_err(|e: io::Error| e.into()))
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
