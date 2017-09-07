use std::io;
use std::sync::{Arc};

use tk_http::Status;
use http_file_headers::{Input as HeadersInput};

use config::static_files::{SingleFile};
use default_error_page::{serve_error_page};
use incoming::{Input, Request, Transport};
use handlers::files::pools::get_pool;
use handlers::files::common::{reply_file, NotFile};


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
    let settings2 = settings.clone();

    let hinp = HeadersInput::from_headers(&settings.headers_config,
        inp.headers.method(), inp.headers.headers());
    let fut = pool.spawn_fn(move || {
        hinp.probe_file(&settings2.path)
        .map(|x| (x, ()))
        .map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                (NotFile::Status(Status::Forbidden), ())
            } else {
                error!("Error reading file {:?}: {}", settings2.path, e);
                (NotFile::Status(Status::InternalServerError), ())
            }
        })
    });

    reply_file(inp, pool, fut, move |e, ()| {
        if let Some(ref val) = settings.content_type {
            e.add_header("Content-Type", val);
        }
        e.add_extra_headers(&settings.extra_headers);
    })
}
