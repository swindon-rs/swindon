use std::sync::Arc;
use config::ldap::Ldap;
use tk_http::server::{Error};

use incoming::Input;


pub fn check(cfg: &Arc<Ldap>, input: &mut Input)
    -> Result<bool, Error>
{
    unimplemented!();
}
