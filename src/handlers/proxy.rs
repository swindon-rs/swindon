use std::sync::Arc;
use proxy::frontend::Codec;

use minihttp::Status;
use minihttp::server::RequestTarget::Authority;
use tokio_core::io::Io;
use config::proxy::Proxy;
use incoming::{Request, Input};
use default_error_page::serve_error_page;


pub fn serve<S: Io + 'static>(settings: &Arc<Proxy>, inp: Input)
    -> Request<S>
{
    if inp.headers.host().is_none() {
        // Can't proxy without Host
        return serve_error_page(Status::BadRequest, inp)
    }
    if matches!(*inp.headers.request_target(), Authority(..)) {
        // Can't proxy without Host
        return serve_error_page(Status::BadRequest, inp)
    }
    Box::new(Codec::new(settings, inp))
}
