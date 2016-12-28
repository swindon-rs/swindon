use futures::future::Future;
use minihttp::server::{Codec, EncoderDone, Error};

mod input;
mod router;
mod debug;
mod encoder;
mod quick_reply;
mod handler;

pub type Request<S> = Box<Codec<S, ResponseFuture=Reply<S>>>;
pub type Reply<S> = Box<Future<Item=EncoderDone<S>, Error=Error>>;

pub use self::input::Input;
pub use self::router::Router;
pub use self::debug::Debug;
pub use self::encoder::{Encoder, IntoContext};
pub use self::quick_reply::reply;

