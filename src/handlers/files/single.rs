use std::io;
use std::sync::{Arc};

use futures::{Future};
use futures::future::{ok, Either, loop_fn, Loop};
use tk_http::server::Error;
use tk_http::Status;
use http_file_headers::{Input as HeadersInput, Output};

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
    let settings2 = settings.clone();

    let hinp = HeadersInput::from_headers(&settings.headers_config,
        inp.headers.method(), inp.headers.headers());
    let fut = pool.spawn_fn(move || {
        hinp.probe_file(&settings2.path).map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                Status::Forbidden
            } else {
                error!("Error reading file {:?}: {}", settings2.path, e);
                Status::InternalServerError
            }
        })
    });

    reply(inp, move |mut e| {
        Box::new(fut.then(move |result| {
            match result {
                Ok(Output::File(outf)) | Ok(Output::FileRange(outf)) => {
                    if outf.is_partial() {
                        e.status(Status::PartialContent);
                    } else {
                        e.status(Status::Ok);
                    }
                    e.add_length(outf.content_length());
                    for (name, val) in outf.headers() {
                        e.format_header(name, val);
                    }
                    if let Some(ref val) = settings.content_type {
                        e.add_header("Content-Type", val);
                    }
                    e.add_extra_headers(&settings.extra_headers);
                    // add headers
                    if e.done_headers() {
                        // start writing body
                        Either::B(loop_fn((e, outf), move |(mut e, mut outf)| {
                            pool.spawn_fn(move || {
                                outf.read_chunk(&mut e).map(|b| (b, e, outf))
                            }).and_then(|(b, e, outf)| {
                                e.wait_flush(4096).map(move |e| (b, e, outf))
                            }).map(|(b, e, outf)| {
                                if b == 0 {
                                    Loop::Break(e.done())
                                } else {
                                    Loop::Continue((e, outf))
                                }
                            }).map_err(|e| Error::custom(e))
                        }))
                    } else {
                        Either::A(ok(e.done()))
                    }
                }
                Ok(Output::FileHead(head)) | Ok(Output::NotModified(head)) => {
                    if head.is_not_modified() {
                        e.status(Status::NotModified);
                    } else if head.is_partial() {
                        e.status(Status::PartialContent);
                        e.add_length(head.content_length());
                    } else {
                        e.status(Status::Ok);
                        e.add_length(head.content_length());
                    }
                    for (name, val) in head.headers() {
                        e.format_header(name, val);
                    }
                    if let Some(ref val) = settings.content_type {
                        e.add_header("Content-Type", val);
                    }
                    e.add_extra_headers(&settings.extra_headers);
                    assert_eq!(e.done_headers(), false);
                    Either::A(ok(e.done()))
                }
                Ok(Output::InvalidRange) => {
                    Either::A(error_page(
                        Status::RequestRangeNotSatisfiable, e))
                }
                Ok(Output::InvalidMethod) => {
                    Either::A(error_page(
                        Status::MethodNotAllowed, e))
                }
                Ok(Output::NotFound) => {
                    Either::A(error_page(Status::NotFound, e))
                }
                Ok(Output::Directory) => {
                    Either::A(error_page(Status::Forbidden, e))
                }
                Err(status) => {
                    Either::A(error_page(status, e))
                }
            }
        }))
    })
}
