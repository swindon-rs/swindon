use tk_http::{Status};
use tk_http::server::{Error, EncoderDone};
use trimmer::{Template, Context, Variable, Var, DataError};

use template;
use futures::future::{ok, FutureResult};
use incoming::{reply, Request, Encoder, IntoContext};

#[derive(Debug)]
pub struct StatusVar(Status);


lazy_static! {
    static ref TEMPLATE: Template = template::PARSER.parse(
        include_str!("default_error_page.html"))
        .expect("default error page is a valid template");
}

pub fn serve_error_page<S: 'static, C: IntoContext>(status: Status, ctx: C)
    -> Request<S>
{
    reply(ctx, move |e| Box::new(error_page(status, e)))
}

pub fn error_page<S: 'static>(status: Status, mut e: Encoder<S>)
    -> FutureResult<EncoderDone<S>, Error>
{
    e.status(status);
    if status.response_has_body() {
        let status_var = StatusVar(status);
        let mut ctx = Context::new();
        ctx.set("status".into(), &status_var);
        let body = match TEMPLATE.render(&ctx) {
            Ok(body) => body,
            Err(e) => {
                error!("Error rendering error page for {:?}: {}", status, e);
                "Error rendering error page".into()
            }
        };
        e.add_length(body.as_bytes().len() as u64);
        e.add_header("Content-Type", "text/html");
        if e.done_headers() {
            e.write_body(body);
        }
    } else {
        e.done_headers();
    }
    ok(e.done())
}

impl<'a> Variable<'a> for StatusVar {
    fn typename(&self) -> &'static str {
        "Status"
    }
    fn attr<'x>(&'x self, attr: &str)
        -> Result<Var<'x, 'a>, DataError>
        where 'a: 'x
    {
        match attr {
            "code" => Ok(Var::owned(self.0.code())),
            "reason" => Ok(Var::str(self.0.reason())),
            _ => Err(DataError::AttrNotFound),
        }
    }
}
