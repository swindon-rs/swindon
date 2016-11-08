use std::sync::Arc;
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

use futures::{BoxFuture, Future, Async, finished};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use tokio_service::Service;
use tk_bufstream::IoBuf;
use minihttp::{Error, GenericResponse, ResponseWriter, Status, Request};
use minihttp::enums::Method;
use minihttp::client::HttpClient;
use httpbin::HttpBin;

use config::{Config, EmptyGif};
use config::static_files::{Static, SingleFile};

use response::DebugInfo;
use default_error_page::error_page;
use handlers::{files, empty_gif, proxy};
use handlers::proxy::ProxyCall;
use chat;
use websocket;
use {Pickler};


pub struct Serializer {
    config: Arc<Config>,
    debug: DebugInfo,
    response: Response,
    handle: Remote,
    request: Option<Request>,
}

pub enum Response {
    ErrorPage(Status),
    EmptyGif(Arc<EmptyGif>),
    HttpBin,
    Static {
        path: PathBuf,
        settings: Arc<Static>,
    },
    SingleFile(Arc<SingleFile>),
    WebsocketEcho(websocket::Init),
    WebsocketChat(chat::ChatInit),
    Proxy(ProxyCall),
}

impl Response {
    pub fn serve(self, req: Request, cfg: Arc<Config>,
                 debug: DebugInfo, handle: &Handle,
                 http_client: &HttpClient)
        -> BoxFuture<Serializer, Error>
    {
        use self::Response::*;
        use super::chat::ChatInit::*;
        match self {
            Proxy(ProxyCall::Prepare{ hostport, settings}) => {
                let handle = handle.remote().clone();

                let mut client = http_client.clone();
                proxy::prepare(req, hostport, settings, &mut client);

                client.done()
                    .map_err(|e| e.into())
                    .and_then(move |resp| {
                        let resp = Response::Proxy(ProxyCall::Ready {
                            response: resp,
                        });
                        finished(Serializer {
                            config: cfg,
                            debug: debug,
                            response: resp,
                            handle: handle,
                            request: None,
                        })
                    }).boxed()
            }
            WebsocketChat(Prepare(init, router)) => {
                let remote = handle.remote().clone();
                let client = http_client.clone();

                let url = router.get_url("tangle.authorize_connection".into());
                let http_cookies = req.headers.iter()
                    .filter(|&&(ref k, _)| k == "Cookie")
                    .map(|&(_, ref v)| v.clone())
                    .collect::<String>();
                let http_auth = req.headers.iter()
                    .find(|&&(ref k, _)| k == "Authorization")
                    .map(|&(_, ref v)| v.clone());
                // println!("Cookies: {:?}; {:?}", http_cookies, http_auth);

                let mut auth = http_client.clone();
                auth.request(Method::Post, url.as_str());
                // TODO: write auth message with cookies
                //  get request's headers (cookie & authorization)
                //  TODO: connection with id;
                auth.add_header("Content-Type".into(), "application/json");
                auth.add_length(0);
                auth.done_headers();
                auth.done()
                .map_err(|e| e.into())
                .and_then(move |resp| {
                    let resp = if resp.status == Status::Ok {
                        match chat::parse_response(resp.status, resp.body) {
                            Ok(userinfo) => {
                                WebsocketChat(
                                    Ready(init, client, router, userinfo))
                            }
                            Err(err) => {
                                WebsocketChat(AuthError(init, err))
                            }
                        }
                    } else {
                        Response::ErrorPage(Status::InternalServerError)
                    };
                    finished(Serializer {
                        config: cfg,
                        debug: debug,
                        response: resp,
                        handle: remote,
                        request: None,
                    })
                })
                .boxed()
            }
            _ => {
                finished(Serializer {
                    config: cfg,
                    debug: debug,
                    response: self,
                    handle: handle.remote().clone(),
                    request: Some(req), // only needed for HttpBin
                }).boxed()
            }
        }
    }
}

impl<S: Io + AsRawFd + Send + 'static> GenericResponse<S> for Serializer {
    type Future = BoxFuture<IoBuf<S>, Error>;
    fn into_serializer(mut self, writer: ResponseWriter<S>) -> Self::Future {
        use super::chat::ChatInit::*;
        let writer = Pickler(writer, self.config, self.debug);
        match self.response {
            Response::ErrorPage(status) => {
                error_page(status, writer)
            }
            Response::EmptyGif(cfg) => {
                empty_gif::serve(writer, cfg)
            }
            Response::HttpBin => {
                // TODO(tailhook) it's not very good idea to unpack the future
                // this way
                match HttpBin::new().call(self.request.take().unwrap()).poll()
                {
                    Ok(Async::Ready(gen_response)) => {
                        gen_response.into_serializer(writer.0).boxed()
                    }
                    _ => unreachable!(),
                }
            }
            Response::Static { path, settings } => {
                files::serve(writer, path, settings)
            }
            Response::SingleFile(settings) => {
                files::serve_file(writer, settings)
            }
            Response::WebsocketEcho(init) => {
                websocket::negotiate(writer, init, self.handle,
                    websocket::Kind::Echo)
            }
            Response::WebsocketChat(Ready(init, client, router, userinfo)) => {
                chat::negotiate(writer, init, self.handle, client,
                    router, userinfo)
            }
            Response::WebsocketChat(_) => {
                error_page(Status::BadRequest, writer)
            }
            Response::Proxy(ProxyCall::Ready { response }) => {
                proxy::serialize(writer, response)
            }
            Response::Proxy(_) => {
                error_page(Status::BadRequest, writer)
            }
        }
    }
}
