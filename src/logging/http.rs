use std::fmt;
use std::net::SocketAddr;

use tk_http::Status;
use tk_http::server::Head;
use trimmer::{Variable, Var, DataError, Output};

use request_id::RequestId;
use logging::context::{Context, AsContext};


pub struct EarlyRequest<'a> {
    pub addr: SocketAddr,
    pub head: &'a Head<'a>,
    pub request_id: RequestId,
}

#[derive(Debug)]
pub struct EarlyResponse {
    pub status: Status,
}

#[derive(Debug)]
pub struct FakeResponse {
}

// Temporary page object
pub struct FakePage<'a> {
    pub request: EarlyRequest<'a>,
    pub response: FakeResponse,
}

pub struct EarlyError<'a> {
    pub request: EarlyRequest<'a>,
    pub response: EarlyResponse,
}

#[derive(Debug)]
pub struct Display<D: fmt::Display + fmt::Debug>(D);

impl<'a> AsContext for EarlyError<'a> {
    fn as_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.set("request", &self.request);
        ctx.set("response", &self.response);
        ctx
    }
}

impl<'a> AsContext for FakePage<'a> {
    fn as_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.set("request", &self.request);
        ctx.set("response", &self.response);
        ctx
    }
}

impl<'a> Variable<'a> for EarlyRequest<'a> {
    fn attr<'x>(&'x self, attr: &str) -> Result<Var<'x, 'a>, DataError>
        where 'a: 'x
    {
        match attr {
            // TODO(tailhook) return just IP when trimmer is updated
            "client_ip" => Ok(Var::owned(self.addr.ip())),
            "host" => Ok(Var::owned(
                self.head.host()
                .unwrap_or("-")
                .to_string())),
            "method" => Ok(Var::owned(self.head.method())),
            "path" => Ok(Var::owned(self.head.path())),
            "version" => Ok(Var::owned(Display(self.head.version()))),
            _ => Err(DataError::AttrNotFound),
        }
    }
    fn typename(&self) -> &'static str {
        "EarlyRequest"
    }
}

impl<'a> fmt::Debug for EarlyRequest<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EarlyRequest")
         .finish()
    }
}

impl<'a> Variable<'a> for EarlyResponse {
    fn attr<'x>(&'x self, attr: &str) -> Result<Var<'x, 'a>, DataError>
        where 'a: 'x
    {
        match attr {
            "status_code" => Ok(Var::owned(self.status.code())),
            _ => Err(DataError::AttrNotFound),
        }
    }
    fn typename(&self) -> &'static str {
        "EarlyResponse"
    }
}

impl<'a> Variable<'a> for FakeResponse {
    fn attr<'x>(&'x self, attr: &str) -> Result<Var<'x, 'a>, DataError>
        where 'a: 'x
    {
        match attr {
            "status_code" => Ok(Var::str("non-404")),
            _ => Err(DataError::AttrNotFound),
        }
    }
    fn typename(&self) -> &'static str {
        "FakeResponse"
    }
}

impl<'a, D: fmt::Display + fmt::Debug + 'a> Variable<'a> for Display<D> {
    fn as_bool(&self) -> Result<bool, DataError> {
        Ok(true)
    }
    fn output(&self) -> Result<Output, DataError> {
        Ok((&self.0).into())
    }
    fn typename(&self) -> &'static str {
        "Display"
    }
}
