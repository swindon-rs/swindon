use std::sync::Arc;
use std::marker::PhantomData;

use futures::{Async, Future};
use futures::future::FutureResult;
use tokio_core::io::Io;
use tokio_core::reactor::Handle;
use minihttp::Status;
use minihttp::server::{Dispatcher, Error, Head};
use minihttp::server as http;
use minihttp::server::{EncoderDone, RecvMode, WebsocketAccept};

use intern::{Topic, Lattice as Namespace};
use chat::Cid;
use config::SessionPool;
use runtime::Runtime;

pub struct Handler {
    runtime: Arc<Runtime>,
    settings: Arc<SessionPool>,
    handle: Handle,
}

pub struct Request {
    settings: Arc<SessionPool>,
    query: Result<Route, Status>,
}

pub enum Route {
    /// `PUT /v1/connection/<conn_id>/subscriptions/<path>`
    Subscribe(Cid, Topic),
    /// `DELETE /v1/connection/<conn_id>/subscriptions/<path>`
    Unsubscribe(Cid, Topic),
    /// `POST /v1/publish/<path>`
    Publish(Topic),
    /// `PUT /v1/connection/<conn_id>/lattices/<namespace>`
    Attach(Cid, Namespace),
    /// `DELETE /v1/connection/<conn_id>/lattices/<namespace>`
    Detach(Cid, Namespace),
    /// `POST /v1/lattice/<namespace>`
    Lattice(Namespace),
}

impl Handler {
    pub fn new(runtime: Arc<Runtime>, settings: Arc<SessionPool>,
        handle: Handle)
        -> Handler
    {
        Handler {
            runtime: runtime,
            settings: settings,
            handle: handle,
        }
    }
}

impl<S: Io> Dispatcher<S> for Handler {
    type Codec = Request;
    fn headers_received(&mut self, headers: &Head)
        -> Result<Self::Codec, Error>
    {
        let query = match headers.path() {
            Some(path) => {
                if !path.starts_with("/v1/") {
                    Err(Status::NotFound)
                } else {
                    self.dispatch(&path[4..], headers.method())
                }
            }
            None => {
                Err(Status::BadRequest)
            }
        };
        Ok(Request {
            settings: self.settings.clone(),
            query: query,
        })
    }
}

impl Handler {
    fn dispatch(&mut self, path: &str, method: &str) -> Result<Route, Status> {
        let mut iter = path.splitn(2, '/');
        let head = iter.next().unwrap();
        let tail = iter.next();
        match (head, tail) {
            ("connection", Some(tail)) => {
                let mut p = tail.splitn(3, '/');
                let cid = p.next().and_then(|x| x.parse().ok());
                let middle = p.next();
                let tail = p.next();
                match middle {
                    Some("subscriptions") => {
                        let topic = tail.and_then(|x| {
                            x.replace("/", ".").parse().ok()
                        });
                        match (method, cid, topic) {
                            ("PUT", Some(cid), Some(t)) => {
                                Ok(Route::Subscribe(cid, t))
                            }
                            ("DELETE", Some(cid), Some(t)) => {
                                Ok(Route::Unsubscribe(cid, t))
                            }
                            _ => Err(Status::NotFound),
                        }
                    }
                    Some("lattices") => {
                        let ns = tail.and_then(|x| {
                            x.replace("/", ".").parse().ok()
                        });
                        match (method, cid, ns) {
                            ("PUT", Some(cid), Some(ns)) => {
                                Ok(Route::Attach(cid, ns))
                            }
                            ("DELETE", Some(cid), Some(ns)) => {
                                Ok(Route::Detach(cid, ns))
                            }
                            _ => Err(Status::NotFound),
                        }
                    }
                    _ => Err(Status::NotFound),
                }
            }
            ("publish", Some(tail)) => {
                let topic = tail.replace("/", ".").parse().ok();
                if let Some(topic) = topic {
                    Ok(Route::Publish(topic))
                } else {
                    Err(Status::NotFound)
                }
            }
            ("lattice", Some(tail)) => {
                let topic = tail.replace("/", ".").parse().ok();
                if let Some(topic) = topic {
                    Ok(Route::Lattice(topic))
                } else {
                    Err(Status::NotFound)
                }
            }
            _ => {
                Err(Status::NotFound)
            }
        }
    }
}

impl<S: Io> http::Codec<S> for Request {
    type ResponseFuture = FutureResult<EncoderDone<S>, Error>;
    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::BufferedUpfront(self.settings.max_payload_size)
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        assert!(end);
        unimplemented!();
    }
    fn start_response(&mut self, e: http::Encoder<S>) -> Self::ResponseFuture {
        unimplemented!();
    }
}
