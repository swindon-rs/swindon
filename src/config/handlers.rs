use std::sync::Arc;

use quire::validate::{Enum, Nothing};

use super::chat;
use super::static_files;
use super::proxy;


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub enum Handler {
    SwindonChat(chat::Chat),
    Static(Arc<static_files::Static>),
    Proxy(Arc<proxy::Proxy>),
    EmptyGif,
    /// This endpoints is for testing websocket implementation. It's not
    /// guaranteed to work in forward compatible manner. We use it for
    /// autobahn tests, but we might choose to change test suite, so don't use
    /// it for something serious.
    WebsocketEcho,
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("SwindonChat", chat::validator())
    .option("Static", static_files::validator())
    .option("Proxy", proxy::validator())
    .option("EmptyGif", Nothing)
    .option("WebsocketEcho", Nothing)
}
