use std::io::BufWriter;
use std::sync::Arc;

use futures::future::{ok};
use libcantal::{Json, Collection};
use self_meter_http::{ThreadReport, ProcessReport};
use serde_json;
use tk_http::Status;

use config::self_status::SelfStatus;
use incoming::{reply, Request, Input};
use metrics;


pub fn serve<S: 'static>(settings: &Arc<SelfStatus>, inp: Input)
    -> Request<S>
{
    let settings = settings.clone();
    let meter = inp.runtime.meter.clone();
    let fingerprint = inp.runtime.config.fingerprint();
    reply(inp, move |mut e| {

        #[derive(Serialize)]
        struct Response<'a> {
            process: ProcessReport<'a>,
            threads: ThreadReport<'a>,
            metrics: Json<'a, Vec<Box<Collection>>>,
            config_fingerprint: String,
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
                metrics: Json(&metrics::all()),
                config_fingerprint: fingerprint,
            }).expect("report is serializable");
        }
        Box::new(ok(e.done()))
    })
}
