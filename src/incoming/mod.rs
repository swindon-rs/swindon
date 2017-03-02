use std::os::unix::io::AsRawFd;

use tokio_core::io::Io;
use tk_sendfile::Destination;
use futures::future::Future;
use tk_http::server::{Codec, EncoderDone, Error};

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
pub use self::encoder::{Encoder, IntoContext, Context};
pub use self::quick_reply::reply;

/// A transport trait. We currently include ``AsRawFd`` in it to allow
/// sendfile to work. But in the future we want to use specialization
/// to optimize sendfile
pub trait Transport: Io + AsRawFd + Destination + 'static {}
impl<T: Io + AsRawFd + Destination + 'static> Transport for T {}
