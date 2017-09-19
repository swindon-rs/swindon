use std::sync::Arc;

use quire::validate::{Enum, Nothing};

use config::ldap;
use config::networks;


#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Authorizer {
    AllowAll,
    SourceIp(Arc<networks::SourceIpAuthorizer>),
    Ldap(Arc<ldap::Ldap>),
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("AllowAll", Nothing)
    .option("Ldap", ldap::authorizer_validator())
    .option("SourceIp", networks::source_ip_authorizer_validator())
}
