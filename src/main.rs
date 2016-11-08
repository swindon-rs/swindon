#[macro_use] extern crate log;
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
use minihttp::client::HttpClient;

use config::{ListenSocket, Handler};
use handler::Main;
pub use response::Pickler;


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
    let handler = Main {
        config: cfg.clone(),
        handle: lp.handle(),
        http_client: HttpClient::new(lp.handle()),
    };
    // TODO(tailhook) do something when config updates
    for sock in &cfg.get().listen {
        match sock {
            &ListenSocket::Tcp(addr) => {
                if verbose {
                    println!("Listening at {}", addr);
                }
                minihttp::serve(&lp.handle(), addr, handler.clone());
            }
        }
    }
    for (name, h) in cfg.get().handlers.iter() {
        match h {
            &Handler::SwindonChat(ref chat) => {
                match chat.listen {
                    ListenSocket::Tcp(addr) => {
                        if verbose {
                            println!("Listening {} at {}", name, addr);
                            // TODO: start Chat API handler;
                            //  bound to its own sub-config;
                            // handler can hold connection storage ref;
                            //  and use it to get connection
                        }
                    }
                }
            }
            _ => {}
        }
    }
    handlers::files::update_pools(&cfg.get().disk_pools);

    let config_updater = Interval::new(Duration::new(10, 0), &lp.handle())
        .expect("interval created")
        .for_each(move |_| {
            match configurator.try_update() {
                Ok(false) => {}
                Ok(true) => {
                    // TODO(tailhook) update listening sockets
                    info!("Updated config");
                    handlers::files::update_pools(&cfg.get().disk_pools);
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
            Ok(())
        });

    lp.run(config_updater).unwrap();
}
