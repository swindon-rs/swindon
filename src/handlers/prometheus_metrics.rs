use std::io::{self, Write, BufWriter};
use std::time::Instant;

use futures::future::{ok};
// use self_meter_http::{ThreadReport, ProcessReport};
use tk_http::Status;

use crate::incoming::{reply, Request, Input};
use crate::prometheus_metrics::{self, Collection, Visit, Value};


pub fn serve<S: 'static>(inp: Input) -> Request<S> {

    // let meter = inp.runtime.meter.clone();
    let fingerprint = inp.runtime.config.fingerprint();
    let runtime = inp.runtime.clone();
    let start = Instant::now();

    reply(inp, move |mut e| {

        e.status(Status::Ok);
        e.add_header("Content-Type", "text/plain; version=0.0.4");
        e.add_chunked();
        if e.done_headers() {
            let mut w = BufWriter::new(&mut e);

            // TODO: serialize meter (process & threads reports)

            to_writer(&mut w, Info(fingerprint));
            to_writer(&mut w, prometheus_metrics::all(&runtime));

            write!(w, "# metrics render microseconds: {}\n",
                start.elapsed().as_micros())
            .expect("duration can be formatted");
        }
        Box::new(ok(e.done()))
    })
}


fn to_writer<W, T: Collection>(writer: &mut W, collection: T)
    where W: Write,
{
    collection.visit(&mut TextEncoder(writer, ""));
}


struct TextEncoder<'a, W: Write> (&'a mut W, &'static str);

impl<'a, W: Write> TextEncoder<'a, W> {
    fn write(&mut self, name: &'static str, labels: &[(&str, &str)], value: &dyn Value)
        -> io::Result<()>
    {
        if self.1 != name {
            self.1 = name;
            write!(self.0, "# TYPE {} {}\n", self.1, value.type_name())?;
        }
        if !labels.is_empty() {
            for (i, (label, value)) in labels.iter().enumerate() {
                match i {
                    0 => {
                        write!(self.0, "{}{{{}=\"{}\"", self.1, label, value)?;
                    }
                    _ => {
                        write!(self.0, ",{}=\"{}\"", label, value)?;
                    }
                }
            }
            write!(self.0, "}} {}\n", value)
        } else {
            write!(self.0, "{} {}\n", self.1, value)
        }
    }
}

impl<'a, W: Write> Visit for TextEncoder<'a, W> {
    fn metric(&mut self, name: &'static str, labels: &[(&str, &str)], value: &dyn Value) {
        self.write(name, labels, value)
        .map_err(|e| error!("error rendering metric: {}", e))
        .ok();
    }
}

struct Info(String);

impl prometheus_metrics::Collection for Info {
    fn visit(&self, visitor: &mut dyn prometheus_metrics::Visit) {
        visitor.info_metric("info", &[("version", env!("CARGO_PKG_VERSION"))]);
        visitor.info_metric("info", &[("config_fingerprint", &self.0)]);
    }
}
