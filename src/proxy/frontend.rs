use std::sync::Arc;
use std::mem;

use futures::{Async, Future, AsyncSink};
use futures::future::{ok};
use futures::sink::{Sink};
use futures::sync::oneshot;
use minihttp::Status;
use minihttp::server::{Error, RecvMode};
use minihttp::server as http;
use tokio_core::io::Io;

use config::proxy::Proxy;
use incoming::{Input, Debug, Reply, Encoder, Context, IntoContext};
use default_error_page::error_page;
use http_pools::HttpPools;
use proxy:: {RepReq, HalfReq, Response, backend};


enum State {
    Headers(HalfReq),
    Sent {
        /// Mostly to resend the request
        request: RepReq,
        response: oneshot::Receiver<Response>,
    },
    Error(Status),
    Void,
}


pub struct Codec {
    settings: Arc<Proxy>,
    pools: HttpPools,
    state: State,
    context: Option<Context>,
}

impl<S: Io + 'static> http::Codec<S> for Codec {
    type ResponseFuture = Reply<S>;
    fn recv_mode(&mut self) -> RecvMode {
        if self.settings.stream_requests {
            unimplemented!();
        } else {
            RecvMode::BufferedUpfront(self.settings.max_payload_size)
        }
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        if self.settings.stream_requests {
            unimplemented!();
        }
        self.state = match mem::replace(&mut self.state, State::Void) {
            State::Error(e) => State::Error(e),
            State::Headers(r) => {
                assert!(end);
                let r = r.upgrade(data.to_vec());
                let mut up = self.pools.upstream(
                    &self.settings.destination.upstream);
                let (tx, rx) = oneshot::channel();
                let codec = Box::new(backend::Codec::new(r.clone(), tx));
                match up.get_mut().get_mut() {
                    Some(pool) => {
                        match pool.start_send(codec) {
                            Ok(AsyncSink::NotReady(r)) => {
                                State::Error(Status::ServiceUnavailable)
                            }
                            Ok(AsyncSink::Ready) => {
                                debug!("Sent request {:?} to proxy", r);
                                State::Sent {
                                    request: r,
                                    response: rx,
                                }
                            }
                            Err(e) => {
                                error!("Error sending to pool {:?}: {}",
                                    self.settings.destination.upstream, e);
                                State::Error(Status::InternalServerError)
                            }
                        }
                    }
                    None => {
                        error!("No such pool {:?}",
                            self.settings.destination.upstream);
                        State::Error(Status::NotFound)
                    }
                }
            }
            State::Sent { .. } => unimplemented!(),
            State::Void => unreachable!(),
        };
        return Ok(Async::Ready(data.len()));
    }
    fn start_response(&mut self, e: http::Encoder<S>) -> Reply<S> {
        if self.settings.stream_requests {
            unimplemented!();
        } else {
            let ctx = self.context.take().unwrap();
            match mem::replace(&mut self.state, State::Void) {
                State::Sent { response, .. } => {
                    Box::new(response.then(move |result| {
                        let e = Encoder::new(e, ctx);
                        match result {
                            Ok(resp) => {
                                ok(resp.encode(e))
                            }
                            Err(err) => {
                                debug!("Proxy request error: {:?}", err);
                                error_page(Status::BadGateway, e)
                            }
                        }
                    }))
                }
                State::Error(status) => {
                    Box::new(error_page(status, Encoder::new(e, ctx)))
                }
                _ => unreachable!(),
            }
        }
    }
}

impl Codec {
    pub fn new(settings: &Arc<Proxy>, inp: Input) -> Codec {
        Codec {
            settings: settings.clone(),
            pools: inp.runtime.http_pools.clone(),
            state: State::Headers(HalfReq::from_input(&inp)),
            context: Some(inp.into_context()),
        }
    }
}
