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
use super::websocket::ImmediateReplier;
use {Pickler};

pub mod handler;
mod message;
mod proto;
mod router;
mod processor;

pub use self::processor::Processor;
pub use self::proto::{Chat, parse_userinfo};
pub use self::router::MessageRouter;
pub use self::message::{Message, Meta, Args, Kwargs, MessageError};

/// Internal connection id
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Cid(u64);

pub enum ChatInit {
    Prepare(Init, MessageRouter),
    AuthError(Init, Message),
    Ready(Init, HttpClient, MessageRouter, Message),
}

pub fn negotiate<S>(mut response: Pickler<S>, init: Init, remote: Remote,
    http_client: HttpClient, router: MessageRouter, userinfo: Message)
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
            ImmediateReplier::new(&mut socket.out_buf)
                .text(userinfo.encode().as_str());

            let user_id = userinfo.get_user_id().unwrap().clone();
            let dispatcher = Chat::new(
                handle.clone(), http_client, router, user_id);
            WebsockProto::new(socket, dispatcher, handle)
            .map_err(|e| info!("Websocket error: {}", e))
        });
        Err(io::Error::new(io::ErrorKind::BrokenPipe,
                           "Connection is stolen for websocket"))
    })
    .map_err(|e: io::Error| e.into())
    .boxed()
}

impl Cid {
    #[cfg(target_pointer_width = "64")]
    pub fn new() -> Cid {
        // Until atomic u64 really works
        use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
        static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        Cid(COUNTER.fetch_add(1, Ordering::Relaxed) as u64)
    }
}
