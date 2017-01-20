use std::fmt;
use std::mem;
use std::str::from_utf8;
use std::sync::Arc;
use std::net::SocketAddr;

use futures::{Async, Future};
use futures::future::{FutureResult, ok};
use tokio_core::io::Io;
use tokio_core::reactor::Handle;
use minihttp::Status;
use minihttp::server::{Dispatcher, Error, Head};
use minihttp::server as http;
use minihttp::server::{EncoderDone, RecvMode, WebsocketAccept};
use rustc_serialize::json;

use intern::{Topic, SessionPoolName, Lattice as Namespace};
use chat::Cid;
use chat::processor::Action;
use chat::listener::spawn::WorkerData;
use config::SessionPool;
use runtime::Runtime;


pub struct Handler {
    addr: SocketAddr,
    wdata: Arc<WorkerData>,
}

pub enum State {
    Query(Route),
    Done,
    Error(Status),
}

pub struct Request {
    wdata: Arc<WorkerData>,
    state: State,
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

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Route::*;
        match *self {
            Subscribe(cid, ref tpc) => {
                write!(f, "Subscribe {:#?} {:?}", cid, tpc)
            }
            Unsubscribe(cid, ref tpc) => {
                write!(f, "Unsubscribe {:#?} {:?}", cid, tpc)
            }
            Publish(ref topic) => write!(f, "Publish {:?}", topic),
            Attach(cid, ref ns) => {
                write!(f, "Lattice attach {:#?} {:?}", cid, ns)
            }
            Detach(cid, ref ns) => {
                write!(f, "Lattice detach {:#?} {:?}", cid, ns)
            }
            Lattice(ref ns) => write!(f, "Lattice update {:?}", ns),
        }
    }
}

impl Handler {
    pub fn new(addr: SocketAddr, wdata: Arc<WorkerData>)
        -> Handler
    {
        Handler {
            addr: addr,
            wdata: wdata,
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
                    State::Error(Status::NotFound)
                } else {
                    self.dispatch(&path[4..], headers.method())
                }
            }
            None => {
                State::Error(Status::BadRequest)
            }
        };
        match query {
            State::Query(ref route) => {
                info!("{:?} received {} (ip: {})",
                    self.wdata.name, route, self.addr);
            }
            State::Done => unreachable!(),
            State::Error(status) => {
                info!("{:?} path {:?} gets {:?} (ip: {})",
                    self.wdata.name, headers.path(), status, self.addr);
            }
        }
        Ok(Request {
            wdata: self.wdata.clone(),
            state: query,
        })
    }
}

impl Handler {
    fn dispatch(&mut self, path: &str, method: &str) -> State {
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
                                State::Query(Route::Subscribe(cid, t))
                            }
                            ("DELETE", Some(cid), Some(t)) => {
                                State::Query(Route::Unsubscribe(cid, t))
                            }
                            _ => State::Error(Status::NotFound),
                        }
                    }
                    Some("lattices") => {
                        let ns = tail.and_then(|x| {
                            x.replace("/", ".").parse().ok()
                        });
                        match (method, cid, ns) {
                            ("PUT", Some(cid), Some(ns)) => {
                                State::Query(Route::Attach(cid, ns))
                            }
                            ("DELETE", Some(cid), Some(ns)) => {
                                State::Query(Route::Detach(cid, ns))
                            }
                            _ => State::Error(Status::NotFound),
                        }
                    }
                    _ => State::Error(Status::NotFound),
                }
            }
            ("publish", Some(tail)) => {
                let topic = tail.replace("/", ".").parse().ok();
                if let Some(topic) = topic {
                    State::Query(Route::Publish(topic))
                } else {
                    State::Error(Status::NotFound)
                }
            }
            ("lattice", Some(tail)) => {
                let topic = tail.replace("/", ".").parse().ok();
                if let Some(topic) = topic {
                    State::Query(Route::Lattice(topic))
                } else {
                    State::Error(Status::NotFound)
                }
            }
            _ => {
                State::Error(Status::NotFound)
            }
        }
    }
}

impl<S: Io> http::Codec<S> for Request {
    type ResponseFuture = FutureResult<EncoderDone<S>, Error>;
    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::BufferedUpfront(self.wdata.settings.max_payload_size)
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        use self::Route::*;
        assert!(end);
        let query = mem::replace(&mut self.state,
                                 State::Error(Status::InternalServerError));
        self.state = match query {
            State::Query(Subscribe(cid, topic)) => {
                if data.len() == 0 {
                    self.wdata.processor.send(Action::Subscribe {
                        conn_id: cid,
                        topic: topic,
                    });
                    State::Done
                } else {
                    State::Error(Status::BadRequest)
                }
            }
            State::Query(Unsubscribe(cid, topic)) => {
                if data.len() == 0 {
                    self.wdata.processor.send(Action::Unsubscribe {
                        conn_id: cid,
                        topic: topic,
                    });
                    State::Done
                } else {
                    State::Error(Status::BadRequest)
                }
            }
            State::Query(Publish(topic)) => {
                // TODO(tailhook) check content-type
                let data = from_utf8(data)
                    .map_err(|e| {
                        info!("Error decoding utf-8 for '/v1/publish': \
                            {:?}", e);
                    })
                    .and_then(|data| json::Json::from_str(data)
                    .map_err(|e| {
                        info!("Error decoding json for '/v1/publish': \
                            {:?}", e);
                    }));
                match data {
                    Ok(json) => {
                        self.wdata.processor.send(Action::Publish {
                            topic: topic,
                            data: Arc::new(json),
                        });
                        State::Done
                    }
                    Err(_) => {
                        State::Error(Status::BadRequest)
                    }
                }
            }
            State::Query(Attach(cid, ns)) => {
                unimplemented!();
            }
            State::Query(Detach(cid, ns)) => {
                unimplemented!();
            }
            State::Query(Lattice(ns)) => {
                unimplemented!();
            }
            State::Done => unreachable!(),
            State::Error(e) => State::Error(e),
        };
        Ok(Async::Ready(data.len()))
    }
    fn start_response(&mut self, mut e: http::Encoder<S>)
        -> Self::ResponseFuture
    {
        if let State::Error(status) = self.state {
            e.status(status);
            // TODO(tailhook) add some body describing the error
            e.done_headers().unwrap();
            ok(e.done())
        } else {
            e.status(Status::NoContent);
            e.done_headers().unwrap();
            ok(e.done())
        }
    }
}
