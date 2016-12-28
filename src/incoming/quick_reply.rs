use std::sync::Arc;

use tokio_core::io::Io;
use futures::Async;
use minihttp::server::{EncoderDone, Error, Codec, RecvMode};
use minihttp::server as http;

use config::Config;
use incoming::{Request, Reply, Encoder, IntoContext, Debug};


pub struct QuickReply<F> {
    inner: Option<(F, Arc<Config>, Debug)>,
}


pub fn reply<F, C, S: Io + 'static>(ctx: C, f: F)
    -> Request<S>
    where F: FnOnce(Encoder<S>) -> Reply<S> + 'static,
          C: IntoContext,
{
    let (cfg, debug) = ctx.into_context();
    Box::new(QuickReply {
        inner: Some((f, cfg, debug)),
    })
}

impl<F, S: Io> Codec<S> for QuickReply<F>
    where F: FnOnce(Encoder<S>) -> Reply<S>,
{
    type ResponseFuture = Reply<S>;
    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::BufferedUpfront(0)
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        assert!(end);
        assert!(data.len() == 0);
        Ok(Async::Ready(0))
    }
    fn start_response(&mut self, mut e: http::Encoder<S>) -> Reply<S> {
        let (func, config, debug) = self.inner.take()
            .expect("start response called once");
        func(Encoder::new(e, config, debug))
    }
}
