use std::io;
use std::ascii::AsciiExt;

use sha1::Sha1;
use futures::{BoxFuture, Future};
use tokio_core::io::Io;
use tokio_core::reactor::Remote;
use tk_bufstream::IoBuf;

use minihttp::enums::Status;
use minihttp::{Error, Request};

use super::base64::Base64;
use super::proto::WebsockProto;
use super::{Kind};
use super::echo;
use {Pickler};

const GUID: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub struct Init {
    accept: [u8; 20],
}

pub fn prepare(req: &Request) -> Result<Init, Status> {
    let mut upgrade = false;
    let mut connection = false;
    let mut version = false;
    let mut accept = None;
    for &(ref name, ref value) in &req.headers {
        if name == "Sec-WebSocket-Key" {
            if accept.is_some() {
                return Err(Status::BadRequest);
            }
            let mut sha1 = Sha1::new();
            sha1.update(value.trim().as_bytes());
            sha1.update(GUID.as_bytes());
            accept = Some(sha1.digest().bytes());
        } else if name == "Sec-WebSocket-Version" {
            if value != "13" {
                return Err(Status::BadRequest);
            } else {
                version = true;
            }
        } else if name == "Upgrade" {
            if !value.eq_ignore_ascii_case("websocket") {
                return Err(Status::BadRequest);
            } else {
                upgrade = true;
            }
        } else if name == "Connection" {
            if !value.eq_ignore_ascii_case("upgrade") {
                return Err(Status::BadRequest);
            } else {
                connection = true;
            }
        }
        // TODO(tailhook) Sec-WebSocket-Protocol
        // TODO(tailhook) Check transfer-encoding and content-length
    }
    if !upgrade {
        return Err(Status::UpgradeRequired);
    }
    if !connection || !version || accept.is_none() {
        return Err(Status::BadRequest);
    }
    Ok(Init {
        accept: accept.take().unwrap(),
    })
}

#[allow(unreachable_code)]
pub fn negotiate<S>(mut response: Pickler<S>, init: Init, remote: Remote,
    kind: Kind)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static
{
    response.status(Status::SwitchingProtocol);
    response.add_header("Upgrade", "websocket");
    response.add_header("Connection", "upgrade");
    response.format_header("Sec-WebSocket-Accept", Base64(&init.accept[..]));
    response.done_headers();
    response.steal_socket()
    .and_then(move |socket: IoBuf<S>| {
        remote.spawn(move |handle| {
            let dispatcher = match kind {
                Kind::Echo => echo::Echo(handle.clone()),
            };
            WebsockProto::new(socket, dispatcher, handle)
            .map_err(|e| info!("Websocket error: {}", e))
        });
        Err(io::Error::new(io::ErrorKind::BrokenPipe,
                           "Connection is stolen for websocket"))
    })
    .map_err(|e: io::Error| e.into())
    .boxed()
}
