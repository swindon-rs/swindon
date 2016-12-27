use std::io::Write;

use tokio_core::io::Io;

use minihttp::{Status};

pub struct Html {
    status: Status,
    prefix: Arc<PathBuf>,
    data: &'static str,
}

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


pub fn write_error_page<S>(status: Status, mut response: Pickler<S>)
    -> Pickler<S>
    where S: Io + Send + 'static,
{
    response.status(status);
    if status.response_has_body() {
        let reason = status.reason();
        let content_length = PART1.len() + PART2.len() + PART3.len() +
            2*(4 + reason.as_bytes().len());
        response.add_length(content_length as u64);
        response.add_header("Content-Type", "text/html");
        if response.done_headers() {
            write!(response, "\
                {p1}{code:03} {status}{p2}{code:03} {status}{p3}",
                    code=status.code(), status=status.reason(),
                    p1=PART1, p2=PART2, p3=PART3)
                .expect("writing to a buffer always succeeds");
        }
    } else {
        response.done_headers();
    }
    response
}
