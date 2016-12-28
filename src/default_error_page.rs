use std::io::Write;
use std::sync::Arc;

use minihttp::{Status};
use tokio_core::io::Io;

use futures::future::{ok};
use incoming::{reply, Request, Input};


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


pub fn error_page<S: Io + 'static>(status: Status, inp: Input) -> Request<S> {
    reply(inp, move |mut e| {
        e.status(status);
        if status.response_has_body() {
            let reason = status.reason();
            let content_length = PART1.len() + PART2.len() + PART3.len() +
                2*(4 + reason.as_bytes().len());
            e.add_length(content_length as u64);
            e.add_header("Content-Type", "text/html");
            if e.done_headers() {
                write!(e, "\
                    {p1}{code:03} {status}{p2}{code:03} {status}{p3}",
                        code=status.code(), status=status.reason(),
                        p1=PART1, p2=PART2, p3=PART3)
                    .expect("writing to a buffer always succeeds");
            }
        } else {
            e.done_headers();
        }
        Box::new(ok(e.done()))
    })
}
