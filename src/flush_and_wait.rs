// TODO(tailhook) move to tk_bufstream

use std::time::Duration;

use futures::{Future, Poll, Async};
use tokio_core::reactor::{Handle, Timeout};
use tokio_core::io::Io;

use tk_bufstream::{IoBuf};

/// A future which yields the original stream when output buffer is fully
/// written to the socket
pub struct FlushAndWait<S: Io>{
    sock: IoBuf<S>,
    timeout: Timeout,
}

impl<S: Io> FlushAndWait<S> {
    pub fn new(sock: IoBuf<S>, handle: &Handle, timeout: Duration)
        -> FlushAndWait<S>
    {
        FlushAndWait {
            sock: sock,
            timeout: Timeout::new(timeout, handle).unwrap(),
        }
    }
}


impl<S: Io> Future for FlushAndWait<S> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<(), ()> {
        if self.sock.done() {
            return Ok(Async::Ready(()));
        }
        self.sock.flush().map_err(|_| ())?;
        // TODO(tailhook) shutdown connection
        Ok(self.timeout.poll().map_err(|_| ())?)
    }
}
