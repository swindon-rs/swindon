use std::sync::Arc;

use tokio_core::io::Io;
use minihttp::server::{Head};

use incoming::{Request, Input};
use config::{Config, Handler};
use handlers;

// TODO(tailhook) this should eventually be a virtual method on Handler trait
impl Handler {
    pub fn serve<S>(&self, input: Input) -> Request<S>
        where S: Io + 'static
    {
        match *self {
            Handler::EmptyGif(ref h) => {
                handlers::empty_gif::serve(h, input)
            }
            Handler::HttpBin => {
                unimplemented!();
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
