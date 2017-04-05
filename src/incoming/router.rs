use std::net::SocketAddr;
use std::sync::Arc;

use tokio_core::reactor::Handle;
use tk_http::Status;
use tk_http::server::{Dispatcher, Error, Head};

use runtime::Runtime;
use incoming::{Request, Debug, AuthInput, Input, Transport};
use routing::{parse_host, route};
use default_error_page::serve_error_page;
use request_id;


pub struct Router {
    addr: SocketAddr,
    runtime: Arc<Runtime>,
    handle: Handle,
}

impl Router {
    pub fn new(addr: SocketAddr, runtime: Arc<Runtime>, handle: Handle)
        -> Router
    {
        Router {
            addr: addr,
            runtime: runtime,
            handle: handle,
        }
    }
}

impl<S: Transport> Dispatcher<S> for Router {
    type Codec = Request<S>;
    fn headers_received(&mut self, headers: &Head)
        -> Result<Self::Codec, Error>
    {
        // Keep config same while processing a single request
        let cfg = self.runtime.config.get();
        let request_id = request_id::new();
        let mut debug = Debug::new(headers, request_id, &cfg);

        // No path means either CONNECT host, or OPTIONS *
        // in both cases we use root route for the domain to make decision
        //
        // TODO(tailhook) strip ?, #, ; from path
        let path = headers.path().unwrap_or("/");

        let parsed_host = headers.host().map(parse_host);

        let authorization_route = parsed_host
            .and_then(|host| route(host, &path, &cfg.authorization));

        if let Some((auth, pref, suf)) = authorization_route {
            debug.set_authorizer(auth);
            let mut inp = AuthInput {
                addr: self.addr,
                runtime: &self.runtime,
                config: &cfg,
                debug: debug,
                headers: headers,
                prefix: pref,
                suffix: suf,
                handle: &self.handle,
                request_id: request_id,
            };
            if let Some(authorizer) = cfg.authorizers.get(auth) {
                match authorizer.check(&mut inp) {
                    Ok(true) => {}
                    Ok(false) => {
                        return Ok(serve_error_page(Status::Forbidden, inp));
                    }
                    Err(e) => return Err(e),
                }
            } else {
                error!("Can't find authorizer {}. Forbiddng request.", auth);
                inp.debug.set_deny("authorizer-not-found");
                return Ok(serve_error_page(Status::Forbidden, inp));
            }
            debug = inp.debug;
        };

        let matched_route = parsed_host
            .and_then(|host| route(host, &path, &cfg.routing));

        let (handler, pref, suf) = if let Some((route, p, s)) = matched_route {
            debug.set_route(route);
            (cfg.handlers.get(route), p, s)
        } else {
            (None, "", path)
        };
        let inp = Input {
            addr: self.addr,
            runtime: &self.runtime,
            config: &cfg,
            debug: debug,
            headers: headers,
            prefix: pref,
            suffix: suf,
            handle: &self.handle,
            request_id: request_id,
        };
        if let Some(handler) = handler {
            handler.serve(inp)
        } else {
            Ok(serve_error_page(Status::NotFound, inp))
        }
    }
}
