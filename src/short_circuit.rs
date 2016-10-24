use std::mem;

use futures::{Future, Poll};
use futures::Async::{Ready, NotReady};


#[allow(dead_code)]
pub enum ShortCircuit<F>
    where F: Future,
{
    Future(F),
    Value(Result<F::Item, F::Error>),
    #[doc(hidden)]
    Done,
}


impl<F: Future> Future for ShortCircuit<F> {
    type Item = F::Item;
    type Error = F::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use self::ShortCircuit::*;
        let future = match mem::replace(self, Done) {
            Future(mut f) => match try!(f.poll()) {
                Ready(v) => return Ok(Ready(v)),
                NotReady => f,
            },
            Value(v) => {
                return Ok(Ready(try!(v)))
            }
            Done => unreachable!(),
        };
        *self = ShortCircuit::Future(future);
        Ok(NotReady)
    }
}
