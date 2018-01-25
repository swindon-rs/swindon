use tk_http::server::{Error};

use futures::{Future, Async, IntoFuture};
use futures::future::ok;
use incoming::input::{AuthInput};
use config::{Authorizer};
use authorizers;

#[must_use="futures don't do anything unless polled"]
pub struct AuthFuture {
    future: Box<Future<Item=bool, Error=Error>>,
}

// TODO(tailhook) this should eventually be a virtual method on Authorizer
impl Authorizer {
    pub fn check(&self, input: &mut AuthInput) -> AuthFuture {
        match *self {
            Authorizer::AllowAll => {
                return AuthFuture {
                    future: Box::new(ok(true)),
                }
            }
            Authorizer::SourceIp(ref cfg) => {
                return AuthFuture {
                    future: Box::new(
                        authorizers::source_ip::check(cfg, input)
                        .into_future()
                    ),
                };
            }
            Authorizer::Ldap(ref cfg) => {
                return AuthFuture {
                    future: authorizers::ldap::check(cfg, input),
                }
            }
        }
    }
}

impl Future for AuthFuture {
    type Item = bool;
    type Error = Error;
    fn poll(&mut self) -> Result<Async<bool>, Error> {
        self.future.poll()
    }
}
