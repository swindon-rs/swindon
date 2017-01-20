use std::sync::Arc;

use futures::{Async, Future};
use futures::stream::{Stream};
use futures::sink::{Sink};
use minihttp::Status;
use minihttp::server::{EncoderDone, Error, Codec, RecvMode, WebsocketAccept};
use minihttp::server as http;
use minihttp::websocket::{Codec as WebsocketCodec, Packet};
use tk_bufstream::{ReadBuf, WriteBuf};
use tokio_core::io::Io;
use futures::future::{ok};
use futures::sync::oneshot::{channel, Receiver};
use tokio_core::reactor::Handle;
use rustc_serialize::json::{self, Json};

use chat;
use intern::SessionId;
use config::Config;
use config::chat::Chat;
use incoming::{Request, Input, Debug, Reply, Encoder, Transport};
use incoming::{Context, IntoContext};
use default_error_page::{serve_error_page, error_page};

struct ReplyData {
    context: Context,
    accept: WebsocketAccept,
    authorizer: Receiver<Result<Arc<Json>, Status>>,
}

struct WebsockReply {
    rdata: Option<ReplyData>,
    user_info: Option<Receiver<Result<Arc<Json>, Status>>>,
    handle: Handle,
}


impl<S: Io + 'static> Codec<S> for WebsockReply {
    type ResponseFuture = Reply<S>;
    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::Hijack
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        unreachable!();
    }
    fn start_response(&mut self, mut e: http::Encoder<S>) -> Reply<S> {
        let (tx, rx) = channel();
        self.user_info = Some(rx);
        let ReplyData { context, accept, authorizer } = self.rdata.take()
            .expect("start response called once");
        Box::new(authorizer.then(move |result| {
            let mut e = Encoder::new(e, context);
            // We always allow websocket, and send error as shutdown message
            // in case there is one.
            let msg = match result {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(status)) => Err(status),
                Err(_) => {
                    error!("authentication future is cancelled");
                    Err(Status::InternalServerError)
                }
            };
            tx.complete(msg);

            e.status(Status::SwitchingProtocol);
            e.add_header("Connection", "upgrade");
            e.add_header("Upgrade", "websocket");
            e.format_header("Sec-Websocket-Accept", &accept);
            e.done_headers();
            ok(e.done())
        }))
    }
    fn hijack(&mut self, write_buf: WriteBuf<S>, read_buf: ReadBuf<S>) {
        let uchannel = self.user_info.take().unwrap();
        let inp = read_buf.framed(WebsocketCodec);
        let out = write_buf.framed(WebsocketCodec);
        // TODO(tailhook) convert Ping to Pong (and Close ?) before echoing
        let fut = uchannel.then(move |x| match x {
            Ok(Ok(auth_data)) => {
                let msg = chat::ConnectionMessage::Hello(auth_data);
                out.send(Packet::Text(json::encode(&msg)
                    .expect("every message can be encoded")))
                .map_err(|e| info!("error sending userinfo: {:?}", e))
                .and_then(|out| inp.forward(out)
                    .map_err(|e| info!("error sending userinfo: {:?}", e))
                    .map(|(_, _)| debug!("websocket complete")))

            }
            Ok(Err(_)) => {
                // TODO(tailhook) shutdown gracefully
                unimplemented!();
            }
            Err(_) => {
                error!("authentication (userinfo) future is cancelled");
                // TODO(tailhook) shutdown gracefully
                unimplemented!();
            }
        });
        self.handle.spawn(fut);
    }
}

pub fn serve<S: Transport>(settings: &Arc<Chat>, inp: Input)
    -> Result<Request<S>, Error>
{
    match inp.headers.get_websocket_upgrade() {
        Ok(Some(ws)) => {
            let (tx, rx) = channel();
            chat::start_authorize(&inp, settings, tx);
            Ok(Box::new(WebsockReply {
                handle: inp.handle.clone(),
                rdata: Some(ReplyData {
                    context: inp.into_context(),
                    accept: ws.accept,
                    authorizer: rx,
                }),
                user_info: None,
            }))
        }
        Ok(None) => {
            if let Some(ref hname) = settings.http_route {
                if let Some(handler) = inp.config.handlers.get(hname) {
                    handler.serve(inp)
                } else {
                    warn!("No such handler for `http-route`: {:?}", hname);
                    Ok(serve_error_page(Status::NotFound, inp))
                }
            } else {
                Ok(serve_error_page(Status::NotFound, inp))
            }
        }
        Err(()) => {
            Ok(serve_error_page(Status::BadRequest, inp))
        }
    }
}
