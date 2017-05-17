use std::sync::Arc;
use std::mem;

use futures::{Async, Future, AsyncSink};
use futures::future::{ok};
use futures::sink::{Sink};
use futures::sync::oneshot;
use tk_http::Status;
use tk_http::server::{Error, RecvMode};
use tk_http::server as http;

use config::proxy::Proxy;
use incoming::{Input, Reply, Encoder, Context, IntoContext};
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

impl<S: 'static> http::Codec<S> for Codec {
    type ResponseFuture = Reply<S>;
    fn recv_mode(&mut self) -> RecvMode {
        if self.settings.stream_requests {
            unimplemented!();
        } else {
            RecvMode::buffered_upfront(self.settings.max_payload_size)
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
                let dest_name = &self.settings.destination.upstream;
                let mut up = self.pools.upstream(dest_name);
                let (tx, rx) = oneshot::channel();
                let ref cfg = self.context.as_ref().unwrap().0;
                let opt_dest = cfg.http_destinations.get(dest_name);
                if let Some(dest_settings) = opt_dest {
                    let codec = Box::new(backend::Codec::new(r.clone(),
                        dest_settings, tx));
                    match up.get_mut().get_mut() {
                        Some(pool) => {
                            match pool.start_send(codec) {
                                Ok(AsyncSink::NotReady(_)) => {
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
                } else {
                    error!("No such destination {:?}",
                        self.settings.destination.upstream);
                    State::Error(Status::NotFound)
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
            state: State::Headers(HalfReq::from_input(&inp, &settings)),
            pools: inp.runtime.http_pools.clone(),
            settings: settings.clone(),
            context: Some(inp.into_context()),
        }
    }
}
