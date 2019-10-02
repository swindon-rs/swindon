use std::io::BufWriter;
use std::sync::Arc;

use futures::future::{ok};
use libcantal::{Json, Collection};
use self_meter_http::{ThreadReport, ProcessReport};
use serde_json;
use tk_http::Status;

use crate::config::self_status::SelfStatus;
use crate::incoming::{reply, Request, Input};
use crate::metrics;


pub fn serve<S: 'static>(settings: &Arc<SelfStatus>, inp: Input)
    -> Request<S>
{
    let settings = settings.clone();
    let meter = inp.runtime.meter.clone();
    let fingerprint = inp.runtime.config.fingerprint();
    let runtime = inp.runtime.clone();

    reply(inp, move |mut e| {

        #[derive(Serialize)]
        struct Response<'a> {
            process: ProcessReport<'a>,
            threads: ThreadReport<'a>,
            metrics: Json<'a, Vec<Box<dyn Collection>>>,
            config_fingerprint: String,
            version: &'a str,
        }

        e.status(Status::Ok);
        e.add_chunked();
        if !settings.overrides_content_type {
            e.add_header("Content-Type", "application/json");
        }
        e.add_extra_headers(&settings.extra_headers);
        if e.done_headers() {
            serde_json::to_writer(BufWriter::new(&mut e), &Response {
                process: meter.process_report(),
                threads: meter.thread_report(),
                metrics: Json(&metrics::all(&runtime)),
                config_fingerprint: fingerprint,
                version: env!("CARGO_PKG_VERSION"),
            }).expect("report is serializable");
        }
        Box::new(ok(e.done()))
    })
}
