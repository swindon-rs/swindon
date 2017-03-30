use std::sync::Arc;

use tk_http::Status;
use futures::future::{ok};

use config::empty_gif::EmptyGif;
use incoming::{reply, Request, Input};


const EMPTY_GIF: &'static [u8] = include_bytes!("../empty.gif");


pub fn serve<S: 'static>(settings: &Arc<EmptyGif>, inp: Input)
    -> Request<S>
{
    let settings = settings.clone();
    reply(inp, move |mut e| {
        e.status(Status::Ok);
        e.add_length(EMPTY_GIF.len() as u64);
        if !settings.overrides_content_type {
            e.add_header("Content-Type", "image/gif");
        }
        e.add_extra_headers(&settings.extra_headers);
        if e.done_headers() {
            e.write_body(EMPTY_GIF);
        }
        Box::new(ok(e.done()))
    })
}
