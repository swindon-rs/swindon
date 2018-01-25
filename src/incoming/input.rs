use std::sync::Arc;
use std::net::SocketAddr;

use tk_http::server::Head;
use tokio_core::reactor::Handle;

use config::Config;
use runtime::Runtime;
use incoming::{Debug, IntoContext};
use incoming::authorizer::AuthFuture;
use request_id::RequestId;

pub struct AuthInput<'a> {
    pub addr: SocketAddr,
    pub runtime: &'a Arc<Runtime>,
    pub config: &'a Arc<Config>,
    pub debug: Debug,
    pub headers: &'a Head<'a>,
    pub prefix: &'a str,
    pub suffix: &'a str,
    pub handle: &'a Handle,
    pub request_id: RequestId,
}

pub struct Input<'a> {
    pub addr: SocketAddr,
    pub runtime: &'a Arc<Runtime>,
    pub config: &'a Arc<Config>,
    pub debug: Debug,
    pub headers: &'a Head<'a>,
    pub prefix: &'a str,
    pub suffix: &'a str,
    pub handle: &'a Handle,
    pub request_id: RequestId,
    pub auth_future: AuthFuture,
}

impl<'a> Input<'a> {
    pub fn from_auth(inp: AuthInput<'a>, auth: AuthFuture) -> Input {
        Input {
            addr: inp.addr,
            runtime: inp.runtime,
            config: inp.config,
            debug: inp.debug,
            headers: inp.headers,
            prefix: inp.prefix,
            suffix: inp.suffix,
            handle: inp.handle,
            request_id: inp.request_id,
            auth_future: auth,
        }
    }
}

impl<'a> IntoContext for Input<'a> {
    fn into_context(self) -> (Arc<Config>, Debug) {
        (self.config.clone(), self.debug)
    }
}
