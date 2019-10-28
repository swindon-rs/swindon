use std::path::Path;

use tk_http::server::{Dispatcher, Error};
use tk_http::Status;
use httpbin::HttpBin;

use crate::config::{Handler};
use crate::handlers;
use crate::incoming::{Request, Input, Transport};
use crate::default_error_page::serve_error_page;


// TODO(tailhook) this should eventually be a virtual method on Handler trait
impl Handler {
    pub fn serve<S>(&self, input: Input) -> Result<Request<S>, Error>
        where S: Transport
    {
        match *self {
            Handler::EmptyGif(ref h) => {
                Ok(handlers::empty_gif::serve(h, input))
            }
            Handler::NotFound => {
                Ok(serve_error_page(Status::NotFound, input))
            }
            Handler::HttpBin => {
                HttpBin::new_at(&Path::new(
                    if input.prefix == "" { "/" } else { input.prefix }))
                .instantiate(input.addr)
                .headers_received(input.headers)
            }
            Handler::Static(ref settings) => {
                Ok(handlers::files::serve_dir(settings, input))
            }
            Handler::SingleFile(ref settings) => {
                Ok(handlers::files::serve_file(settings, input))
            }
            Handler::VersionedStatic(ref settings) => {
                Ok(handlers::files::serve_versioned(settings, input))
            }
            Handler::WebsocketEcho => {
                Ok(handlers::websocket_echo::serve(input))
            }
            Handler::Proxy(ref settings) => {
                Ok(handlers::proxy::serve(settings, input))
            }
            Handler::SwindonLattice(ref settings) => {
                handlers::swindon_chat::serve(settings, input)
            }
            Handler::BaseRedirect(ref settings) => {
                Ok(handlers::redirect::base_redirect(settings, input))
            }
            Handler::StripWWWRedirect => {
                Ok(handlers::redirect::strip_www_redirect(input))
            }
            Handler::SelfStatus(ref settings) => {
                Ok(handlers::self_status::serve(settings, input))
            }
        }
    }
}
