use std::mem;
use std::sync::Arc;

use futures::Async;
use futures::future::{FutureResult, ok};
use futures::sync::oneshot;
use tk_http::client as http;

use config::http_destinations::Destination;
use proxy::{RepReq, HalfResp, Response};

enum State {
    Init(RepReq),
    Wait,
    Headers(HalfResp),
    #[allow(dead_code)]
    Done(Response),
    Void,
}


pub struct Codec {
    state: State,
    destination: Arc<Destination>,
    sender: Option<oneshot::Sender<Response>>,
}

impl Codec {
    pub fn new(req: RepReq, destination: &Arc<Destination>,
        tx: oneshot::Sender<Response>)
        -> Codec
    {
        Codec {
            state: State::Init(req),
            destination: destination.clone(),
            sender: Some(tx),
        }
    }
}

impl<S> http::Codec<S> for Codec {
    type Future = FutureResult<http::EncoderDone<S>, http::Error>;

    fn start_write(&mut self, e: http::Encoder<S>) -> Self::Future {
        if let State::Init(req) = mem::replace(&mut self.state, State::Void) {
            self.state = State::Wait;
            ok(req.encode(e, &self.destination))
        } else {
            panic!("wrong state");
        }
    }
    fn headers_received(&mut self, headers: &http::Head)
        -> Result<http::RecvMode, http::Error>
    {
        if let State::Wait = mem::replace(&mut self.state, State::Void) {
            self.state = State::Headers(HalfResp::from_headers(headers));
            // TODO(tailhook) limit and streaming
            Ok(http::RecvMode::buffered(10_485_760))
        } else {
            panic!("wrong state");
        }
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, http::Error>
    {
        // TODO(tailhook) streaming
        assert!(end);
        match mem::replace(&mut self.state, State::Void) {
            State::Headers(hr) => {
                let resp = hr.complete(data.to_vec());
                self.sender.take().unwrap().send(resp).ok();
            }
            _ => unreachable!(),
        }
        Ok((Async::Ready(data.len())))
    }
}
