use std::str;
use std::ascii::AsciiExt;
use futures::{Stream, Future, Async};
use futures::future::{ok};
use futures::sync::mpsc::unbounded;
use tokio_core::reactor::Handle;
use tokio_io::{AsyncRead, AsyncWrite};
use tk_http::Status;
use tk_http::server::{Dispatcher, Head, Error, Codec, RecvMode, Encoder};
use tk_http::websocket::{Accept, Loop, ServerCodec, Config as WsConfig};
use tk_bufstream::{WriteBuf, ReadBuf};

use incoming::{Request, Reply, Transport};
use runtime::ServerId;
use super::spawn::Handler;
use super::{IncomingChannel, ReplAction};
use chat::replication::{CONNECTIONS, FRAMES_SENT};


/// Incoming requests dispatcher.
pub struct Incoming {
    sender: IncomingChannel,
    handle: Handle,
    server_id: ServerId,
}

struct WebsocketCodec {
    sender: IncomingChannel,
    handle: Handle,
    accept: Accept,
    server_id: ServerId,
    remote_id: ServerId,
}

impl Incoming {

    pub fn new(sender: IncomingChannel, server_id: ServerId, handle: &Handle)
        -> Incoming
    {
        Incoming {
            sender: sender,
            handle: handle.clone(),
            server_id: server_id,
        }
    }

    fn parse_remote_id(&self, headers: &Head) -> Option<ServerId>
    {
        headers.all_headers().iter()
        .find(|h| h.name.eq_ignore_ascii_case("X-Swindon-Node-Id"))
        .and_then(|h| str::from_utf8(h.value).ok())
        .and_then(|s| s.parse().ok())
    }
}


impl<S: Transport> Dispatcher<S> for Incoming {
    type Codec = Request<S>;

    fn headers_received(&mut self, headers: &Head)
        -> Result<Self::Codec, Error>
    {
        if let Some("/v1/swindon-chat") = headers.path() {
            if let Ok(Some(ws)) = headers.get_websocket_upgrade() {
                if let Some(remote_id) = self.parse_remote_id(headers)
                {
                    if remote_id == self.server_id {
                        error!(concat!("Configuration error:",
                            " got connection from ourself: {:?}"),
                            self.server_id);
                        return Ok(error_reply(Status::BadRequest));
                    }
                    Ok(Box::new(WebsocketCodec {
                        sender: self.sender.clone(),
                        accept: ws.accept,
                        server_id: self.server_id,
                        remote_id: remote_id,
                        handle: self.handle.clone(),
                    }))
                } else {
                    Ok(error_reply(Status::BadRequest))
                }
            } else {
                Ok(error_reply(Status::BadRequest))
            }
        } else {
            Ok(error_reply(Status::NotFound))
        }
    }
}

impl<S: AsyncRead + AsyncWrite + 'static> Codec<S> for WebsocketCodec {
    type ResponseFuture = Reply<S>;

    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::hijack()
    }

    fn data_received(&mut self, _data: &[u8], _end: bool)
        -> Result<Async<usize>, Error>
    {
        unreachable!()
    }

    fn start_response(&mut self, mut e: Encoder<S>) -> Reply<S> {
        e.status(Status::SwitchingProtocol);
        e.add_header("Connection", "upgrade").unwrap();
        e.add_header("Upgrade", "websocket").unwrap();
        e.format_header("Sec-Websocket-Accept", &self.accept).unwrap();
        e.format_header("X-Swindon-Node-Id", &self.server_id).unwrap();
        e.done_headers().unwrap();
        Box::new(ok(e.done()))
    }

    fn hijack(&mut self, write_buf: WriteBuf<S>, read_buf: ReadBuf<S>) {
        let out = write_buf.framed(ServerCodec);
        let inp = read_buf.framed(ServerCodec);
        let wcfg = WsConfig::new().done();

        let (tx, rx) = unbounded();
        let rx = rx
            .map_err(|e| format!("receive error: {:?}", e))
            .map(|x| {
                FRAMES_SENT.incr(1);
                x
            });
        self.sender.send(ReplAction::Attach {
            tx: tx,
            server_id: self.remote_id,
            peer: None,
        }).ok();

        CONNECTIONS.incr(1);
        self.handle.spawn(
            Loop::server(out, inp, rx, Handler(self.sender.clone()), &wcfg,
                         &self.handle)
            .map_err(|e| error!("Websocket loop error: {:?}", e))
        );
    }
}

// Shortcut for error replies

fn error_reply<S: 'static>(status: Status) -> Request<S> {
    Box::new(QuickReply(Some(status)))
}

struct QuickReply(Option<Status>);

impl<S: 'static> Codec<S> for QuickReply {
    type ResponseFuture = Reply<S>;
    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::buffered_upfront(0)
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        assert!(end);
        assert!(data.len() == 0);
        Ok(Async::Ready(0))
    }
    fn start_response(&mut self, mut e: Encoder<S>) -> Reply<S> {
        e.status(self.0.take().expect("start response called once"));
        e.add_length(0).unwrap();
        e.done_headers().unwrap();
        Box::new(ok(e.done()))
    }
}
