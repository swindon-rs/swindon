use std::io;

use futures::{Future, BoxFuture};
use tokio_core::io::Io;
use tokio_core::reactor::Remote;
use minihttp::{Error};
use minihttp::enums::Status;
use minihttp::client::HttpClient;
use tk_bufstream::IoBuf;

use super::websocket::WebsockProto;
use super::websocket::Init;
use {Pickler};

mod message;
mod proto;

pub use self::proto::Chat;


pub fn negotiate<S>(mut response: Pickler<S>, init: Init, remote: Remote,
    http_client: HttpClient)
    -> BoxFuture<IoBuf<S>, Error>
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
            let dispatcher = Chat(handle.clone(), http_client.clone());
            WebsockProto::new(socket, dispatcher, handle)
            .map_err(|e| info!("Websocket error: {}", e))
        });
        Err(io::Error::new(io::ErrorKind::BrokenPipe,
                           "Connection is stolen for websocket"))
    })
    .map_err(|e: io::Error| e.into())
    .boxed()
}
