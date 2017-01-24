use std::sync::Arc;

use futures::{Async, Future};
use futures::stream::{Stream};
use futures::sink::{Sink};
use minihttp::Status;
use minihttp::server::{Error, Codec, RecvMode, WebsocketAccept};
use minihttp::server as http;
use minihttp::websocket::{self, Codec as WebsocketCodec, Packet};
use tk_bufstream::{ReadBuf, WriteBuf};
use tokio_core::io::Io;
use futures::future::{ok};
use futures::sync::mpsc::{UnboundedReceiver as Receiver};
use tokio_core::reactor::Handle;
use rustc_serialize::json;

use chat::{self, Cid, ConnectionMessage, ConnectionSender};
use runtime::Runtime;
use config::chat::Chat;
use incoming::{Request, Input, Reply, Encoder, Transport};
use incoming::{Context, IntoContext};
use default_error_page::serve_error_page;

struct WebsockReply {
    cid: Cid,
    handle: Handle,
    runtime: Arc<Runtime>,
    settings: Arc<Chat>,
    reply_data: Option<ReplyData>,
    channel: Option<(ConnectionSender, Receiver<ConnectionMessage>)>,
}

struct ReplyData {
    context: Context,
    accept: WebsocketAccept,
}


impl<S: Io + 'static> Codec<S> for WebsockReply {
    type ResponseFuture = Reply<S>;
    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::Hijack
    }
    fn data_received(&mut self, _data: &[u8], _end: bool)
        -> Result<Async<usize>, Error>
    {
        unreachable!();
    }
    fn start_response(&mut self, e: http::Encoder<S>) -> Reply<S> {
        let ReplyData { context, accept } = self.reply_data.take()
            .expect("start response called only once");
        let mut e = Encoder::new(e, context);
        // We always allow websocket, and send error as shutdown message
        // in case there is one.
        e.status(Status::SwitchingProtocol);
        e.add_header("Connection", "upgrade");
        e.add_header("Upgrade", "websocket");
        e.format_header("Sec-Websocket-Accept", &accept);
        e.done_headers();
        Box::new(ok(e.done()))
    }
    fn hijack(&mut self, write_buf: WriteBuf<S>, read_buf: ReadBuf<S>) {
        let inp = read_buf.framed(WebsocketCodec);
        let out = write_buf.framed(WebsocketCodec);

        // TODO(tailhook) don't create config on every websocket
        let cfg = websocket::Config::new()
            // TODO(tailhook) change defaults
            .done();
        let pool_settings = self.runtime.config
            .get().session_pools.get(&self.settings.session_pool)
            // TODO(tailhook) may this unwrap crash?
            //                return error code in this case
            .unwrap().clone();
        let processor = self.runtime.session_pools.processor
            // TODO(tailhook) this doesn't check that pool is created
            .pool(&self.settings.session_pool);
        let h1 = self.handle.clone();
        let r1 = self.runtime.clone();
        let s1 = self.settings.clone();
        let cid = self.cid;

        let (tx, rx) = self.channel.take()
            .expect("hijack called only once");

        self.handle.spawn(rx.into_future()
            .then(move |result| match result {
                Ok((auth_data, rx)) => {
                    out.send(Packet::Text(json::encode(&auth_data)
                        .expect("every message can be encoded")))
                    .map_err(|e| info!("error sending userinfo: {:?}", e))
                    .and_then(move |out| {
                        let rx = rx.map(|x| {
                            Packet::Text(json::encode(&x)
                                .expect("any data can be serialized"))
                        }).map_err(|_| -> &str {
                            // There shouldn't be a real-life case for this.
                            // But in case session-pool has been removed from
                            // the config and connection closes, it might
                            // probably happen, we don't care too much of that.
                            error!("outbound channel unexpectedly closed");
                            "outbound channel unexpectedly closed"
                        });
                        websocket::Loop::new(out, inp, rx, chat::Dispatcher {
                            cid: cid,
                            handle: h1,
                            pool_settings: pool_settings.clone(),
                            processor: processor,
                            runtime: r1,
                            settings: s1,
                            channel: tx,
                            }, &cfg)
                        .map_err(|e| debug!("websocket closed: {}", e))
                    })
                }
                Err(_) => {
                    // TODO(tailhook) shutdown gracefully
                    unimplemented!();
                }
            }));
    }
}

pub fn serve<S: Transport>(settings: &Arc<Chat>, inp: Input)
    -> Result<Request<S>, Error>
{
    match inp.headers.get_websocket_upgrade() {
        Ok(Some(ws)) => {
            let (tx, rx) = ConnectionSender::new();
            let cid = Cid::new();
            chat::start_authorize(&inp, cid, settings, tx.clone());
            Ok(Box::new(WebsockReply {
                cid: cid,
                handle: inp.handle.clone(),
                settings: settings.clone(),
                runtime: inp.runtime.clone(),
                reply_data: Some(ReplyData {
                    context: inp.into_context(),
                    accept: ws.accept,
                }),
                channel: Some((tx, rx)),
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
