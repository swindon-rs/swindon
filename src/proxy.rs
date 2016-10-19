use tokio_core::reactor::Handle;
use tokio_curl::Session;

use minihttp::request::Request;


/// Proxy handler
#[derive(Clone)]
pub struct Proxy {
    session: Session,
}

impl Proxy {

    pub fn new(handle: Handle) -> Proxy {
        Proxy {
            session: Session::new(handle),
        }
    }

    pub fn proxy(&self, req: &Request) {
        println!("Proxing {:?}", req);
    }
}
