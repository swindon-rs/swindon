use std::io::Write;

use netbuf::Buf;
use futures::{BoxFuture, Future};
use tokio_core::net::TcpStream;

use minihttp::{Error};

use {Pickler};

const PART1: &'static str = "\
    <!DOCTYPE html>\
    <html>\
        <head>\
            <title>\
    ";
const PART2: &'static str = "\
            </title>\
        </head>\
        <body>\
            <h1>\
    ";
const PART3: &'static str = concat!("\
            </h1>\
            <hr>\
            <p>Yours faithfully,<br>\
                swindon/", env!("CARGO_PKG_VERSION"), "\
            </p>\
        </body>\
    </html>\
    ");

pub fn error_page(code: u16, status: &str, mut response: Pickler)
    -> BoxFuture<(TcpStream, Buf), Error>
{
    let content_length = PART1.len() + PART2.len() + PART3.len() +
        2*(4 + status.as_bytes().len());
    response.status(code, status);
    response.add_length(content_length as u64);
    response.add_header("Content-Type", "text/html");
    if response.done_headers() {
        write!(&mut response, "\
            {p1}{code:03} {status}{p2}{code:03} {status}{p3}",
                code=code, status=status,
                p1=PART1, p2=PART2, p3=PART3)
            .expect("writing to a buffer always succeeds");
    }
    response.done().boxed()
}
