// TODO(tailhook) until we fill all stabs
#![allow(dead_code)]

#[macro_use] extern crate log;
#[macro_use] extern crate matches;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate lazy_static;
extern crate env_logger;
extern crate futures;
extern crate futures_cpupool;
extern crate quire;
extern crate time;
extern crate argparse;
extern crate tokio_core;
extern crate tokio_service;
extern crate minihttp;
extern crate netbuf;
extern crate mime;
extern crate sha1;
extern crate mime_guess;
extern crate tk_sendfile;
extern crate tk_bufstream;
extern crate rustc_serialize;
extern crate tokio_curl;
extern crate curl;
extern crate byteorder;
extern crate httparse;
extern crate httpbin;
extern crate slab;
extern crate string_intern;

mod intern;
mod config;
mod handler;
mod handlers;
mod routing;
mod serializer;
mod default_error_page;
mod response;
mod websocket;
mod chat;
mod startup;

// Utils
mod short_circuit;
mod either;

use std::io::{self, Write};
use std::time::Duration;
use std::process::exit;

use futures::stream::Stream;
use argparse::{ArgumentParser, Parse, StoreTrue, Print};
use tokio_core::reactor::Core;
use tokio_core::reactor::Interval;

pub use response::Pickler;


pub fn main() {
    env_logger::init().unwrap();

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
            "Print some user-friendly startup messages");
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

    if check {
        exit(0);
    }

    let mut lp = Core::new().unwrap();
    let uhandle = lp.handle();
    let mut loop_state = startup::populate_loop(&lp.handle(), &cfg, verbose);

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
}
