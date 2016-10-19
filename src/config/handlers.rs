use std::sync::Arc;

use quire::validate::{Enum, Nothing};

use super::chat;
use super::static_files;
use super::proxy;


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub enum Handler {
    SwindonChat(chat::Chat),
    Static(Arc<static_files::Static>),
    Proxy(proxy::Proxy),
    EmptyGif,
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("SwindonChat", chat::validator())
    .option("Static", static_files::validator())
    .option("Proxy", proxy::validator())
    .option("EmptyGif", Nothing)
}
