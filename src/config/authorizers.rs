use std::sync::Arc;

use quire::validate::{Enum};

use config::ldap;


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub enum Authorizer {
    Ldap(Arc<ldap::Ldap>),
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("Ldap", ldap::authorizer_validator())
}
