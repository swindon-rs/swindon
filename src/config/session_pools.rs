use quire::validate::{Structure};

use super::listen::{self, ListenSocket};

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct SessionPool {
    pub listen: ListenSocket,
}


pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("listen", listen::validator())
}
