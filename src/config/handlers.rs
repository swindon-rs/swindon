use std::sync::Arc;

use quire::validate::{Enum, Nothing};

use super::chat;
use super::empty_gif;
use super::proxy;
use super::redirect;
use super::self_status;
use super::static_files;


#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Handler {
    SwindonLattice(Arc<chat::Chat>),
    Static(Arc<static_files::Static>),
    SingleFile(Arc<static_files::SingleFile>),
    VersionedStatic(Arc<static_files::VersionedStatic>),
    Proxy(Arc<proxy::Proxy>),
    EmptyGif(Arc<empty_gif::EmptyGif>),
    NotFound,
    HttpBin,
    /// This endpoints is for testing websocket implementation. It's not
    /// guaranteed to work in forward compatible manner. We use it for
    /// autobahn tests, but we might choose to change test suite, so don't use
    /// it for something serious.
    WebsocketEcho,
    BaseRedirect(Arc<redirect::BaseRedirect>),
    StripWWWRedirect,
    SelfStatus(Arc<self_status::SelfStatus>),
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("SwindonLattice", chat::validator())
    .option("Static", static_files::validator())
    .option("SingleFile", static_files::single_file())
    .option("VersionedStatic", static_files::versioned_validator())
    .option("Proxy", proxy::validator())
    .option("HttpBin", Nothing)
    .option("EmptyGif", empty_gif::validator())
    .option("WebsocketEcho", Nothing)
    .option("BaseRedirect", redirect::base_redirect())
    .option("StripWWWRedirect", Nothing)
    .option("SelfStatus", self_status::validator())
}
