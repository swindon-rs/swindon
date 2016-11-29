use std::sync::Arc;
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;

use futures::{BoxFuture, Future, Async, finished};
use futures::sync::mpsc::{unbounded as channel};
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Remote};
use tokio_service::Service;
use tk_bufstream::IoBuf;
use minihttp::{Error, GenericResponse, ResponseWriter, Status, Request};
use minihttp::client::HttpClient;
use httpbin::HttpBin;

use config::{Config, EmptyGif};
use config::static_files::{Static, SingleFile};

use response::DebugInfo;
use default_error_page::write_error_page;
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
            WebsocketChat(Prepare(init, chat_api)) => {
                let remote = handle.remote().clone();
                let cid = chat::Cid::new();
                let (tx, rx) = channel();

                chat_api.authorize_connection(&req, cid, tx.clone())
                .map_err(|e| e.into())
                .and_then(move |resp| {
                    let resp = if resp.status != Status::Ok {
                        if resp.status != Status::Forbidden &&
                           resp.status != Status::Unauthorized
                        {
                            warn!("Bad code returned from \
                                authorize_connection: {:?} {}",
                                resp.status, resp.reason);
                        }
                        WebsocketChat(AuthError(init, resp.status))
                    } else {
                        match chat::parse_userinfo(resp) {
                            Ok((sess_id, userinfo)) => {
                                let session_api = chat_api.session_api(
                                    sess_id, cid, userinfo, tx);
                                WebsocketChat(Ready(init, session_api, rx))
                            }
                            Err(e) => {
                                error!("Bad data returned by \
                                    authorize_connection: {}", e);
                                WebsocketChat(AuthError(init,
                                    Status::InternalServerError))
                            }
                        }
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
                write_error_page(status, writer).done().boxed()
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
            Response::WebsocketChat(Prepare(..)) => {
                unreachable!();
            }
            Response::WebsocketChat(Ready(init, session_api, rx)) =>
            {
                chat::negotiate(writer, init, self.handle, session_api, rx)
            }
            Response::WebsocketChat(AuthError(init, code)) => {
                chat::fail(writer, init,
                    websocket::CloseReason::AuthHttp(code as u16))
            }
            Response::Proxy(ProxyCall::Ready { response }) => {
                proxy::serialize(writer, response)
            }
            Response::Proxy(_) => {
                write_error_page(Status::BadRequest, writer)
                .done().boxed()
            }
        }
    }
}
