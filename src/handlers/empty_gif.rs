use std::sync::Arc;

use futures::{BoxFuture, Future};
use tokio_core::io::Io;
use tk_bufstream::IoBuf;

use minihttp::server::{Error};
use minihttp::Status;

use config::EmptyGif;
use {Pickler};


const EMPTY_GIF: &'static [u8] = include_bytes!("../empty.gif");

pub fn serve<S>(mut response: Pickler<S>, settings: Arc<EmptyGif>)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static
{
    response.status(Status::Ok);
    response.add_length(EMPTY_GIF.len() as u64);
    response.add_header("Content-Type", "image/gif");
    response.add_extra_headers(&settings.extra_headers);
    if response.done_headers() {
        response.write_body(EMPTY_GIF);
    }
    response.done().boxed()
}
