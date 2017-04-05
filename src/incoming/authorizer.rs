use tk_http::server::{Error};

use incoming::{AuthInput};
use config::{Authorizer};
use authorizers;

// TODO(tailhook) this should eventually be a virtual method on Authorizer
impl Authorizer {
    pub fn check(&self, input: &mut AuthInput) -> Result<bool, Error> {
        match *self {
            Authorizer::SourceIp(ref cfg) => {
                authorizers::source_ip::check(cfg, input)
            }
            Authorizer::Ldap(_) => unimplemented!(),
        }
    }
}
