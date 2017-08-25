use std::fs::{File, metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc};
use std::str::from_utf8;

use futures::{Future};
use futures::future::{ok, Either, loop_fn, Loop, FutureResult};
use futures_cpupool::{CpuFuture, CpuPool};
use mime::{TopLevel, Mime};
use tk_http::server::Error;
use tk_http::Status;
use http_file_headers::{Input as HeadersInput, Output};

use config::static_files::{Static, Mode};
use default_error_page::{serve_error_page, error_page};
use incoming::{self, Input, Request, Reply, Transport, Encoder, EncoderDone};
use handlers::files::decode::decode_component;
use handlers::files::pools::get_pool;


pub fn reply<S, A, B>(inp: Input, pool: CpuPool,
    fut: CpuFuture<Output, Status>, fn_ok: A, fn_dir: B)
    -> Request<S>
    where S: Transport,
          A: FnOnce(&mut Encoder<S>) + Send + 'static,
          B: FnOnce(Encoder<S>) -> FutureResult<EncoderDone<S>, Error>,
          B: Send + 'static,
{
    incoming::reply(inp, move |mut e| {
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
                    fn_ok(&mut e);
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
                    fn_ok(&mut e);
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
                Ok(Output::NotFound)  => {
                    Either::A(error_page(Status::NotFound, e))
                }
                // TODO(tailhook) implement directory index
                Ok(Output::Directory) => {
                    Either::A(fn_dir(e))
                }
                Err(status) => {
                    Either::A(error_page(status, e))
                }
            }
        }))
    })
}
