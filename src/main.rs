#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate matches;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate scoped_tls;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate abstract_ns;
extern crate argparse;
extern crate blake2;
extern crate byteorder;
extern crate digest;
extern crate digest_writer;
extern crate env_logger;
extern crate futures;
extern crate futures_cpupool;
extern crate generic_array;
extern crate httparse;
extern crate httpbin;
extern crate libc;
extern crate libcantal;
extern crate mime;
extern crate mime_guess;
extern crate netbuf;
extern crate ns_std_threaded;
extern crate quire;
extern crate rand;
extern crate regex;
extern crate rustc_serialize;
extern crate self_meter_http;
extern crate serde;
extern crate sha1;
extern crate slab;
extern crate string_intern;
extern crate time;
extern crate tk_bufstream;
extern crate tk_http;
extern crate tk_listen;
extern crate tk_pool;
extern crate tk_sendfile;
extern crate tokio_core;
extern crate tokio_io;
extern crate trimmer;
extern crate typenum;

mod authorizers;
mod base64;
mod chat;
mod config;
mod default_error_page;
mod handlers;
mod http_pools;  // TODO(tailhook) move to proxy?
mod incoming;
mod intern;
mod logging;
mod metrics;
mod privileges;
mod proxy;
mod request_id;
mod routing;
mod runtime;
mod startup;
mod template;

use std::io::{self, Write};
use std::time::Duration;
use std::env;
use std::process::exit;

use futures::stream::Stream;
use argparse::{ArgumentParser, Parse, StoreTrue, Print};
use tokio_core::reactor::Core;
use tokio_core::reactor::Interval;

fn init_logging() {
    let format = |record: &log::LogRecord| {
        format!("{} [{}] {}: {}", time::now_utc().rfc3339(),
            record.location().module_path(),
            record.level(), record.args())
    };

    let mut builder = env_logger::LogBuilder::new();
    builder.format(format).filter(None, log::LogLevelFilter::Warn);

    if let Ok(value) = env::var("RUST_LOG") {
       builder.parse(&value);
    }

    builder.init().unwrap();
}

pub fn main() {
    init_logging();

    let mut config = String::from("/etc/swindon/main.yaml");
    let mut check = false;
    let mut verbose = false;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Runs a web server");
        ap.refer(&mut config)
          .add_option(&["-c", "--config"], Parse,
            "Configuration file name")
          .metavar("FILE");
        ap.refer(&mut check)
          .add_option(&["-C", "--check-config"], StoreTrue,
            "Check configuration file and exit");
        ap.add_option(&["--version"],
            Print(env!("CARGO_PKG_VERSION").to_string()),
            "Show version");
        ap.refer(&mut verbose)
            .add_option(&["--verbose"], StoreTrue,
            "Print some user-friendly startup messages. \
             With --check-config prints config fingerprint.");
        ap.parse_args_or_exit();
    }

    let mut configurator = match config::Configurator::new(&config) {
        Ok(cfg) => cfg,
        Err(e) => {
            writeln!(&mut io::stderr(), "{}", e).ok();
            exit(1);
        }
    };
    let cfg = configurator.config();

    if verbose {
        println!("Config fingerprint: {}", cfg.fingerprint());
    }
    if check {
        exit(0);
    }

    let metrics = metrics::all();
    let _guard = libcantal::start(&metrics);

    request_id::with_generator(|| {
        let mut lp = Core::new().unwrap();
        let uhandle = lp.handle();
        let mut loop_state = startup::populate_loop(
            &lp.handle(), &cfg, verbose);

        match privileges::drop(&cfg.get()) {
            Ok(()) => {}
            Err(e) => {
                writeln!(&mut io::stderr(),
                    "Can't drop privileges: {}", e).ok();
                exit(2);
            }
        };

        let config_updater = Interval::new(Duration::new(10, 0), &lp.handle())
            .expect("interval created")
            .for_each(move |_| {
                match configurator.try_update() {
                    Ok(false) => {}
                    Ok(true) => {
                        info!("Updated config");
                        startup::update_loop(&mut loop_state, &cfg, &uhandle);
                    }
                    Err(e) => {
                        error!("{}", e);
                    }
                }
                Ok(())
            });

        lp.run(config_updater).unwrap();
    });
}
