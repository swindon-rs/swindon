use std::sync::Arc;

use quire::validate::{Enum};

use config::ldap;
use config::networks;


#[derive(Deserialize, Debug, PartialEq, Eq)]
pub enum Authorizer {
    SourceIp(Arc<networks::SourceIpAuthorizer>),
    Ldap(Arc<ldap::Ldap>),
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("Ldap", ldap::authorizer_validator())
    .option("SourceIp", networks::source_ip_authorizer_validator())
}
