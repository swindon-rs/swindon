// I don't see **any** reason for this waring to enabled. We only build binary
// and most of these warnings do not apply to real visibility of types inside
// the crate
#![allow(private_in_public)]

#[macro_use] extern crate log;
#[macro_use] extern crate matches;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate scoped_tls;
extern crate env_logger;
extern crate futures;
extern crate futures_cpupool;
extern crate quire;
extern crate time;
extern crate argparse;
extern crate tokio_core;
extern crate tk_http;
extern crate netbuf;
extern crate mime;
extern crate sha1;
extern crate mime_guess;
extern crate tk_sendfile;
extern crate tk_bufstream;
extern crate rustc_serialize;
extern crate byteorder;
extern crate httparse;
extern crate httpbin;
extern crate slab;
extern crate string_intern;
extern crate rand;
extern crate tk_pool;
extern crate abstract_ns;
extern crate ns_std_threaded;
extern crate tk_listen;

mod intern;
mod config;
mod runtime;
mod handlers;
mod routing;
mod default_error_page;
mod chat;
mod dev;
mod startup;
mod incoming;
mod http_pools;  // TODO(tailhook) move to proxy?
mod proxy;
mod base64;
mod request_id;

use std::process::exit;
use std::env;

use argparse::{ArgumentParser, Parse, StoreTrue, Print, List, StoreFalse};
use tokio_core::reactor::Core;

//pub use response::Pickler;


pub fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "warn");
    }
    env_logger::init().unwrap();

    let mut verbose = true;
    let mut show_config = false;
    let mut crossdomain = false;
    let mut port = 8000;
    let mut routes = Vec::<dev::Route>::new();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("
            Runs a web server, configured from the command-line, or in other
            words runs swindon in a devd emulation mode (not all devd options
            are supported yet).
        ");
        ap.refer(&mut routes)
            .add_argument("route", List, "
                Add a route. Routes can be:
                (a) `subdomain/path=DEST` -- serves destination on
                    `http://subdomain.devd.io/path`,
                (b) `subdomain=DEST` -- serves destination at root of
                    subdomain,
                (c) `/path=DEST` -- serves destination
                    on `http://localhost/path`,
                (d) `DEST` -- is the same as `/=DEST`.
                Destinations can be:
                (i) `./local/dir` -- serves files from filesystem,
                (ii) `http://host:12345/` -- proxy requests to specified host.
                Note: unlike in original `devd` we currently keep original
                `Host` in requests to proxy.
            ");
        ap.refer(&mut port)
            .add_option(&["-p", "--port"], Parse, "Listen on specified port");
        ap.refer(&mut show_config)
            .add_option(&["--show-config"], StoreTrue,
            "Show config for swindon that is generate by swindon-dev and
             exit");
        ap.refer(&mut crossdomain)
            .add_option(&["--crossdomain"], StoreTrue,
            "Adds `Access-Control-Allow-Origin: *` header");
        ap.add_option(&["--version"],
            Print(env!("CARGO_PKG_VERSION").to_string()),
            "Show version");
        ap.refer(&mut verbose)
            .add_option(&["--quiet"], StoreFalse,
            "Hide some user-friendly startup messages");
        ap.parse_args_or_exit();
    }

    let config = dev::generate_config(port, &routes, crossdomain);
    if show_config {
        print!("{}", config);
        exit(0);
    }

    let cfg = match config::ConfigCell::from_string(
        &config, "<swindon-dev cli>")
    {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!("Unfortunately config we tried to generated is bad: {}. \
                This is a bug. Please report your command-line and run \
                `swindon-dev ... --show-config` to see the actual \
                generated text. ", e);
            exit(2);
        }
    };

    request_id::with_generator(|| {
        let mut lp = Core::new().unwrap();
        let _state = startup::populate_loop(&lp.handle(), &cfg, verbose);
        lp.run(futures::empty::<(), ()>()).unwrap();
    });
}
