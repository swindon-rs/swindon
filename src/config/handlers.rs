use super::chat;
use super::static_files;
use super::proxy;

use quire::validate::{Enum};


#[derive(RustcDecodable)]
pub enum Handler {
    SwindonChat(chat::Chat),
    Static(static_files::Static),
    Proxy(proxy::Proxy),
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("SwindonChat", chat::validator())
    .option("Static", static_files::validator())
    .option("Proxy", proxy::validator())
}
