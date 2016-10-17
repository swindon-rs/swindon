#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;
extern crate env_logger;
extern crate futures;
extern crate quire;
extern crate argparse;
extern crate tokio_core;
extern crate tokio_service;
extern crate minihttp;
extern crate rustc_serialize;

mod config;

use std::io::{self, Write};
use std::time::Duration;
use std::process::exit;

use futures::{Async, Finished};
use futures::stream::Stream;
use argparse::{ArgumentParser, Parse, StoreTrue, Print};
use tokio_core::reactor::Core;
use tokio_core::reactor::Interval;
use tokio_service::Service;
use minihttp::request::Request;
use minihttp::response::Response;

use config::ListenSocket;


#[derive(Clone)]
struct HelloWorld;

impl Service for HelloWorld {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = Finished<Response, io::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let mut resp = req.new_response();
        resp.set_status(204)
            .set_reason("No Content".to_string())
            .header("Content-Length", "0");
        futures::finished(resp)
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}

pub fn main() {
    env_logger::init().unwrap();

    let mut config = String::from("/etc/swindon/main.yaml");
    let mut check = false;
    let mut verbose = false;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Runs tree of processes");
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
    // TODO(tailhook) do something when config updates
    for sock in &cfg.get().listen {
        match sock {
            &ListenSocket::Tcp(addr) => {
                if verbose {
                    println!("Listening at {}", addr);
                }
                minihttp::serve(&lp.handle(), addr, HelloWorld);
            }
        }
    }

    let config_updater = Interval::new(Duration::new(10, 0), &lp.handle())
        .expect("interval created")
        .for_each(move |_| {
            match configurator.try_update() {
                Ok(false) => {}
                Ok(true) => {
                    // TODO(tailhook) update listening sockets
                    info!("Updated config");
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
            Ok(())
        });

    lp.run(config_updater).unwrap();
}
