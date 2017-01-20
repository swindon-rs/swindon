use std::sync::Arc;

use futures::{Async, Future};
use futures::stream::{Stream};
use minihttp::Status;
use minihttp::server::{Error, Codec, RecvMode, WebsocketAccept};
use minihttp::server as http;
use minihttp::websocket::{Codec as WebsocketCodec};
use tk_bufstream::{ReadBuf, WriteBuf};
use tokio_core::io::Io;
use futures::future::{ok};
use tokio_core::reactor::Handle;

use config::Config;
use incoming::{Request, Input, Debug, Reply, Encoder};
use default_error_page::serve_error_page;


struct WebsockReply {
    rdata: Option<(Arc<Config>, Debug, WebsocketAccept)>,
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
        assert!(end);
        assert!(data.len() == 0);
        Ok(Async::Ready(0))
    }
    fn start_response(&mut self, e: http::Encoder<S>) -> Reply<S> {
        let (config, debug, accept) = self.rdata.take()
            .expect("start response called once");
        let mut e = Encoder::new(e, (config, debug));
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
        // TODO(tailhook) convert Ping to Pong (and Close ?) before echoing
        self.handle.spawn(inp.forward(out)
            .map(|_| ())
            // TODO(tailhook) check error reporting
            .map_err(|e| info!("Websocket error: {}", e)))
    }
}

pub fn serve<S: Io + 'static>(inp: Input) -> Request<S> {
    match inp.headers.get_websocket_upgrade() {
        Ok(Some(ws)) => {
            Box::new(WebsockReply {
                rdata: Some((inp.config.clone(), inp.debug, ws.accept)),
                handle: inp.handle.clone(),
            })
        }
        Ok(None) => {
            serve_error_page(Status::NotFound, inp)
        }
        Err(()) => {
            serve_error_page(Status::BadRequest, inp)
        }
    }
}
