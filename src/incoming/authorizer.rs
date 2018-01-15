use tk_http::server::{Error};

use incoming::{Input};
use config::{Authorizer};
use authorizers;

// TODO(tailhook) this should eventually be a virtual method on Authorizer
impl Authorizer {
    pub fn check(&self, input: &mut Input) -> Result<bool, Error> {
        match *self {
            Authorizer::AllowAll => Ok(true),
            Authorizer::SourceIp(ref cfg) => {
                authorizers::source_ip::check(cfg, input)
            }
            Authorizer::Ldap(ref cfg) => {
                authorizers::ldap::check(cfg, input)
            }
        }
    }
}
