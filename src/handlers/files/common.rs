use futures::{Future};
use futures::future::{ok, Either, loop_fn, Loop, FutureResult};
use futures_cpupool::{CpuFuture, CpuPool};
use tk_http::server::Error;
use tk_http::Status;
use http_file_headers::{Output};

use default_error_page::{error_page};
use incoming::{self, Input, Request, Transport, Encoder, EncoderDone};


pub enum NotFile {
    Status(Status),
    Directory(Vec<u8>),
}


pub fn reply_file<S, A>(inp: Input, pool: CpuPool,
    fut: CpuFuture<Output, NotFile>, fn_ok: A)
    -> Request<S>
    where S: Transport,
          A: FnOnce(&mut Encoder<S>) + Send + 'static,
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
                Ok(Output::Directory) => {
                    Either::A(error_page(Status::Forbidden, e))
                }
                Err(NotFile::Status(status)) => {
                    Either::A(error_page(status, e))
                }
                Err(NotFile::Directory(data)) => {
                    e.status(Status::Ok);
                    e.add_length(data.len() as u64);
                    e.add_header("Content-Type", "text/html; charset=utf-8");
                    fn_ok(&mut e);
                    if e.done_headers() {
                        e.write_body(data)
                    }
                    Either::A(ok(e.done()))
                }
            }
        }))
    })
}
