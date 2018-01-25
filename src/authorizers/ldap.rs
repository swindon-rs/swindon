use std::sync::Arc;

use futures::Future;
use config::ldap::Ldap;
use tk_http::server::{Error};


use incoming::AuthInput;


pub fn check(cfg: &Arc<Ldap>, input: &mut AuthInput)
    -> Box<Future<Item=bool, Error=Error>>
{
    unimplemented!();
}
