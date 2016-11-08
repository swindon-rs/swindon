use std::io;

use futures::{Future, BoxFuture};
use tokio_core::io::Io;
use tokio_core::reactor::Remote;
use minihttp::{Error};
use minihttp::enums::Status;
use minihttp::client::HttpClient;
use tk_bufstream::{IoBuf, Buf};
use rustc_serialize::json::Json;

use super::websocket::WebsockProto;
use super::websocket::Init;
use super::websocket::ImmediateReplier;
use {Pickler};

mod message;
mod proto;
mod router;

pub use self::proto::{Chat, parse_response};
pub use self::router::MessageRouter;
pub use self::message::MessageError;


pub enum ChatInit {
    Prepare(Init, MessageRouter),
    AuthError(Init, MessageError),
    Ready(Init, HttpClient, MessageRouter, Json),
}

pub fn negotiate<S>(mut response: Pickler<S>, init: Init, remote: Remote,
    http_client: HttpClient, router: MessageRouter, userinfo: Json)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static
{
    response.status(Status::SwitchingProtocol);
    response.add_header("Upgrade", "websocket");
    response.add_header("Connection", "upgrade");
    response.format_header("Sec-WebSocket-Accept", init.base64());
    response.done_headers();
    response.steal_socket()
    .and_then(move |mut socket: IoBuf<S>| {
        remote.spawn(move |handle| {
            send_hello(&mut socket.out_buf, &userinfo);

            let dispatcher = Chat(
                handle.clone(), http_client, router, userinfo);
            WebsockProto::new(socket, dispatcher, handle)
            .map_err(|e| info!("Websocket error: {}", e))
        });
        Err(io::Error::new(io::ErrorKind::BrokenPipe,
                           "Connection is stolen for websocket"))
    })
    .map_err(|e: io::Error| e.into())
    .boxed()
}

fn send_hello(buf: &mut Buf, data: &Json) {
    let mut replier = ImmediateReplier::new(buf);
    // XXX: encode as tangle response
    replier.text(data.to_string().as_str());
}
