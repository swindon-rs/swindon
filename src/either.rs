use futures::{Future, Poll};

#[allow(dead_code)]
pub enum Either<A, B>
    where A: Future,
          B: Future<Item=A::Item, Error=A::Error>,
{
    A(A),
    B(B),
}


impl<A, B> Future for Either<A, B>
    where A: Future,
          B: Future<Item=A::Item, Error=A::Error>,
{
    type Item = A::Item;
    type Error = A::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use self::Either::*;
        match self {
            &mut A(ref mut future) => future.poll(),
            &mut B(ref mut future) => future.poll(),
        }
    }
}
