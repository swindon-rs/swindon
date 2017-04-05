use std::io::Write;

use tk_http::{Status};
use tk_http::server::{Error, EncoderDone};

use futures::future::{ok, FutureResult};
use incoming::{reply, Request, Input, Encoder, IntoContext};


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
                swindon web server\
            </p>\
        </body>\
    </html>\
    ");


pub fn serve_error_page<S: 'static, C: IntoContext>(status: Status, ctx: C)
    -> Request<S>
{
    reply(ctx, move |e| Box::new(error_page(status, e)))
}

pub fn error_page<S: 'static>(status: Status, mut e: Encoder<S>)
    -> FutureResult<EncoderDone<S>, Error>
{
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
    ok(e.done())
}
