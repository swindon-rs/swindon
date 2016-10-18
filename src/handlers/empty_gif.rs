use netbuf::Buf;
use futures::{BoxFuture, Future};
use tokio_core::net::TcpStream;

use minihttp::{Error, ResponseWriter};

const EMPTY_GIF: &'static [u8] = include_bytes!("../empty.gif");

pub fn serve_empty_gif(mut response: ResponseWriter)
    -> BoxFuture<(TcpStream, Buf), Error>
{
    response.status(200, "OK");
    response.add_length(EMPTY_GIF.len() as u64).unwrap();
    response.add_header("Content-Type", "image/gif").unwrap();
    if response.done_headers().unwrap() {
        response.write_body(EMPTY_GIF);
    }
    response.done().boxed()
}
