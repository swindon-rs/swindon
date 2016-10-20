use std::io;
use std::ascii::AsciiExt;

use futures::{BoxFuture, Future};
use tokio_core::io::Io;
use tk_bufstream::IoBuf;

use minihttp::enums::Status;
use minihttp::{Error, Request};

use {Pickler};

struct Key([u8; 16]);

pub struct Init {
    #[allow(dead_code)]
    key: Key,
}

pub fn prepare(req: &Request) -> Result<Init, Status> {
    let mut upgrade = false;
    let mut connection = false;
    let mut version = false;
    let mut key = None;
    for &(ref name, ref value) in &req.headers {
        if name == "Sec-WebSocket-Key" {
            unimplemented!();
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
        // } else if name.eq_ignore_ascii_case("Connection") {
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
    if !connection || !version || key.is_none() {
        return Err(Status::BadRequest);
    }
    Ok(Init {
        key: key.take().unwrap(),
    })
}

#[allow(unreachable_code)]
pub fn negotiate<S>(mut response: Pickler<S>, _init: Init)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static
{
    response.status(101, "Switching Protocols");
    response.done_headers();
    response.steal_socket()
    .and_then(|_socket: IoBuf<S>| {
        panic!("Websocket!");
        Ok(_socket)
    })
    .map_err(|e: io::Error| e.into())
    .boxed()
}
