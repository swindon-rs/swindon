use std::sync::Arc;
use std::net::SocketAddr;

use tk_http::server::Head;
use tokio_core::reactor::Handle;

use crate::config::Config;
use crate::runtime::Runtime;
use crate::incoming::{Debug, IntoContext};
use crate::request_id::RequestId;


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
}

impl<'a> IntoContext for Input<'a> {
    fn into_context(self) -> (Arc<Config>, Debug) {
        (self.config.clone(), self.debug)
    }
}
