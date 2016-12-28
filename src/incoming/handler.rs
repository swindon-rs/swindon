use std::sync::Arc;
use std::path::Path;

use tokio_core::io::Io;
use minihttp::server::{Head, Dispatcher, Error};
use httpbin::HttpBin;

use incoming::{Request, Input};
use config::{Config, Handler};
use handlers;

// TODO(tailhook) this should eventually be a virtual method on Handler trait
impl Handler {
    pub fn serve<S>(&self, input: Input) -> Result<Request<S>, Error>
        where S: Io + 'static
    {
        match *self {
            Handler::EmptyGif(ref h) => {
                Ok(handlers::empty_gif::serve(h, input))
            }
            Handler::HttpBin => {
                HttpBin::new_at(&Path::new(
                    if input.prefix == "" { "/" } else { input.prefix }))
                .instantiate(input.addr)
                .headers_received(input.headers)
            }
            Handler::Static(ref settings) => {
                unimplemented!();
            }
            Handler::SingleFile(ref settings) => {
                unimplemented!();
            }
            Handler::WebsocketEcho => {
                unimplemented!();
            }
            Handler::Proxy(ref settings) => {
                unimplemented!();
            }
            Handler::SwindonChat(ref chat) => {
                unimplemented!();
            }
        }
    }
}
