use tk_http::server::{Error};

use crate::incoming::{Input};
use crate::config::{Authorizer};
use crate::authorizers;

// TODO(tailhook) this should eventually be a virtual method on Authorizer
impl Authorizer {
    pub fn check(&self, input: &mut Input) -> Result<bool, Error> {
        match *self {
            Authorizer::AllowAll => Ok(true),
            Authorizer::SourceIp(ref cfg) => {
                authorizers::source_ip::check(cfg, input)
            }
            Authorizer::Ldap(_) => unimplemented!(),
        }
    }
}
