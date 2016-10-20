use futures::{BoxFuture, Future};
use tokio_core::io::Io;
use tk_bufstream::IoBuf;

use minihttp::{Error};

use {Pickler};


const EMPTY_GIF: &'static [u8] = include_bytes!("../empty.gif");

pub fn serve<S>(mut response: Pickler<S>)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static
{
    response.status(200, "OK");
    response.add_length(EMPTY_GIF.len() as u64);
    response.add_header("Content-Type", "image/gif");
    if response.done_headers() {
        response.write_body(EMPTY_GIF);
    }
    response.done().boxed()
}
