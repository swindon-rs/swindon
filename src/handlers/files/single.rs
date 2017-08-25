use std::io;
use std::sync::{Arc};

use futures::{Future};
use futures::future::{ok};
use tk_http::server::Error;
use tk_http::Status;

use config::static_files::{SingleFile};
use default_error_page::{serve_error_page, error_page};
use incoming::{Input, Request, Reply, Transport};
use incoming::reply;
use handlers::files::pools::get_pool;


pub fn serve_file<S: Transport>(settings: &Arc<SingleFile>, mut inp: Input)
    -> Request<S>
{
    if !inp.headers.path().is_some() {
        // Star or authority
        return serve_error_page(Status::Forbidden, inp);
    };
    inp.debug.set_fs_path(&settings.path);
    let pool = get_pool(&inp.runtime, &settings.pool);
    let settings = settings.clone();
    unimplemented!();
    /*
    reply(inp, move |mut e| {
        Box::new(pool.open(settings.path.clone())
            .then(move |res| match res {
                Ok(file) => {
                    e.status(Status::Ok);
                    e.add_length(file.size());
                    e.add_header("Content-Type", &settings.content_type);
                    e.add_extra_headers(&settings.extra_headers);
                    if e.done_headers() {
                        Box::new(e.raw_body()
                            .and_then(|raw_body| file.write_into(raw_body))
                            .map(|raw_body| raw_body.done())
                            .map_err(FileError::Sendfile)
                            .map_err(Error::custom))
                        as Reply<_>
                    } else {
                        Box::new(ok(e.done()))
                    }
                }
                Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                    Box::new(error_page(Status::NotFound, e))
                }
                // TODO(tailhook) find out if we want to expose other
                // errors, for example "Permission denied" and "is a directory"
                Err(_) => {
                    // TODO: log error.
                    Box::new(error_page(Status::InternalServerError, e))
                }
            }))
    })
    */
}
