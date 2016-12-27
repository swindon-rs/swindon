use futures::future::Future;
use minihttp::server::{Codec, EncoderDone, Error};

mod router;

pub type Request<S> = Box<Codec<S, ResponseFuture=Reply<S>>>;
pub type Reply<S> = Box<Future<Item=EncoderDone<S>, Error=Error>>;

pub use self::router::Router;

