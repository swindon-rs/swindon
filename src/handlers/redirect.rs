use std::sync::Arc;

use minihttp::Status;
use tokio_core::io::Io;
use futures::future::ok;

use default_error_page::serve_error_page;
use config::redirect::BaseRedirect;
use incoming::{reply, Request, Input};


pub fn base_redirect<S: Io + 'static>(settings: &Arc<BaseRedirect>, inp: Input)
    -> Request<S>
{
    serve_redirect(settings.redirect_to_domain.as_str(), inp)
}


pub fn strip_www_redirect<S: Io + 'static>(inp: Input)
    -> Request<S>
{
    let dest = inp.headers.host()
        .and_then(|host| host.splitn(2, '.').last());
    match dest {
        Some(host) => serve_redirect(host, inp),
        None => serve_error_page(Status::InternalServerError, inp),
    }
}


fn serve_redirect<S: Io + 'static>(host: &str, inp: Input)
    -> Request<S>
{
    // TODO: properly identify request scheme
    let dest = format!("http://{}{}", host, inp.headers.path().unwrap_or("/"));
    reply(inp, move |mut e| {
        e.status(Status::Found);
        e.add_header("Location", dest);
        e.add_length(0);
        if e.done_headers() {
            // TODO: add HTML with redirect link;
            //      link must be url-encoded;
        }
        Box::new(ok(e.done()))
    })
}
