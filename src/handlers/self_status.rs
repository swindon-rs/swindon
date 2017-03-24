use std::io::BufWriter;
use std::sync::Arc;

use tk_http::Status;
use tokio_core::io::Io;
use futures::future::{ok};

use config::self_status::SelfStatus;
use incoming::{reply, Request, Input};


pub fn serve<S: Io + 'static>(settings: &Arc<SelfStatus>, inp: Input)
    -> Request<S>
{
    let settings = settings.clone();
    let meter = inp.runtime.meter.clone();
    reply(inp, move |mut e| {
        e.status(Status::Ok);
        e.add_chunked();
        if !settings.overrides_content_type {
            e.add_header("Content-Type", "application/json");
        }
        e.add_extra_headers(&settings.extra_headers);
        if e.done_headers() {
            meter.serialize(BufWriter::new(&mut e))
        }
        Box::new(ok(e.done()))
    })
}
