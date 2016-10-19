use std::sync::Arc;

use futures::{BoxFuture, Future};
use tokio_core::io::Io;
use minihttp::{Error, Request};
use minihttp::enums::Method;
use tk_bufstream::IoBuf;
use tokio_curl::Session;
use curl::easy::Easy;

use config::proxy::Proxy;
use config::http_destinations::Destination;
use {Pickler};



pub fn serve<S>(mut response: Pickler<S>, session: Session, call: UpstreamCall)
    -> BoxFuture<IoBuf<S>, Error>
    where S: Io + Send + 'static,
{
    session.perform(call.request)
        .map_err(|e| e.into_error().into())
        .and_then(|mut resp| {
            let code = resp.response_code().unwrap();
            response.status(code as u16, "OK");
            response.add_length(0);
            response.done_headers();
            response.done()
        }).boxed()
}


pub struct UpstreamCall {
    pub request: Easy,
    pub settings: Arc<Proxy>,
}

pub fn prepare(req: &Request, dest: &Destination, settings: Arc<Proxy>)
    -> Result<UpstreamCall, Error>
{
    let mut curl = Easy::new();
    match req.method {
        Method::Get => curl.get(true).unwrap(),
        Method::Post => curl.post(true).unwrap(),
        _ => {}
    };
    let hostport = dest.addresses.first().unwrap();
    curl.url(format!("http://{}{}", hostport, req.path).as_str()).unwrap();

    Ok(UpstreamCall {
        request: curl,
        settings: settings,
    })
}
