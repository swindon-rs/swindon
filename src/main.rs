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
use std::process::exit;

use futures::{Async, Finished};
use argparse::{ArgumentParser, Parse, StoreTrue, Print};
use tokio_core::reactor::Core;
use minihttp::server::Message;
use minihttp::request::Request;
use minihttp::response::Response;

use config::ListenSocket;


#[derive(Clone)]
struct HelloWorld;

impl minihttp::server::HttpService for HelloWorld {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = Finished<Message<Response>, io::Error>;

    fn call(&self, req: Request) -> Self::Future {
        println!("REQUEST: {:?}", req);
        let resp = req.new_response();
        // resp.header("Content-Type", "text/html");
        // resp.body(
        //     format!("<h4>Hello world</h4>\n<b>Method: {:?}</b>",
        //             _request.method).as_str());
        // let resp = resp;

        futures::finished(Message::WithoutBody(resp))
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

    let configurator = match config::Configurator::new(&config) {
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
                minihttp::serve(&lp.handle(), addr, || HelloWorld);
            }
        }
    }

    lp.run(futures::empty::<(), ()>()).unwrap();
}
