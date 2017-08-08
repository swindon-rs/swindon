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
mod dev;
mod handlers;
mod http_pools;  // TODO(tailhook) move to proxy?
mod incoming;
mod intern;
mod logging;
mod metrics;
mod proxy;
mod request_id;
mod routing;
mod runtime;
mod startup;
mod template;

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
