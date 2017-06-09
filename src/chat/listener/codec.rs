use std::fmt;
use std::mem;
use std::sync::Arc;
use std::net::SocketAddr;

use futures::Async;
use futures::future::{FutureResult, ok};
use tk_http::Status;
use tk_http::server::{Dispatcher, Error, Head};
use tk_http::server as http;
use tk_http::server::{EncoderDone, RecvMode};
use serde_json::{self, Value as Json};

use intern::{Topic, Lattice as Namespace};
use chat::cid::PubCid;
use chat::processor::Action;
use chat::processor::Delta;
use chat::listener::spawn::WorkerData;
use chat::replication::RemoteAction;


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
    Subscribe(PubCid, Topic),
    /// `DELETE /v1/connection/<conn_id>/subscriptions/<path>`
    Unsubscribe(PubCid, Topic),
    /// `POST /v1/publish/<path>`
    Publish(Topic),
    /// `PUT /v1/connection/<conn_id>/lattices/<namespace>`
    LatticeSubscribe(PubCid, Namespace),
    /// `DELETE /v1/connection/<conn_id>/lattices/<namespace>`
    Detach(PubCid, Namespace),
    /// `POST /v1/lattice/<namespace>`
    Lattice(Namespace),
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Route::*;
        match *self {
            Subscribe(ref cid, ref tpc) => {
                write!(f, "Subscribe {:#?} {:?}", cid.0, tpc)
            }
            Unsubscribe(ref cid, ref tpc) => {
                write!(f, "Unsubscribe {:#?} {:?}", cid.0, tpc)
            }
            Publish(ref topic) => write!(f, "Publish {:?}", topic),
            LatticeSubscribe(ref cid, ref ns) => {
                write!(f, "Lattice subscribe {:#?} {:?}", cid.0, ns)
            }
            Detach(ref cid, ref ns) => {
                write!(f, "Lattice detach {:#?} {:?}", cid.0, ns)
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

impl<S> Dispatcher<S> for Handler {
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
        match (method, head, tail) {
            (_, "connection", Some(tail)) => {
                let mut p = tail.splitn(3, '/');
                let cid = p.next().and_then(|x| x.parse().ok());
                let middle = p.next();
                let tail = p.next();
                match middle {
                    Some("subscriptions") => {
                        let topic = tail.and_then(|x| {
                            if !x.contains('.') {
                                x.replace("/", ".").parse().ok()
                            } else {
                                None
                            }
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
                            if !x.contains('.') {
                                x.replace("/", ".").parse().ok()
                            } else {
                                None
                            }
                        });
                        match (method, cid, ns) {
                            ("PUT", Some(cid), Some(ns)) => {
                                State::Query(Route::LatticeSubscribe(cid, ns))
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
            ("POST", "publish", Some(tail)) => {
                let topic = if !tail.contains('.') {
                    tail.replace("/", ".").parse().ok()
                } else {
                    None
                };
                if let Some(topic) = topic {
                    State::Query(Route::Publish(topic))
                } else {
                    State::Error(Status::NotFound)
                }
            }
            ("POST", "lattice", Some(tail)) => {
                let ns = if !tail.contains('.') {
                    tail.replace("/", ".").parse().ok()
                } else {
                    None
                };
                if let Some(ns) = ns {
                    State::Query(Route::Lattice(ns))
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

impl<S> http::Codec<S> for Request {
    type ResponseFuture = FutureResult<EncoderDone<S>, Error>;
    fn recv_mode(&mut self) -> RecvMode {
        RecvMode::buffered_upfront(self.wdata.settings.max_payload_size)
    }
    fn data_received(&mut self, data: &[u8], end: bool)
        -> Result<Async<usize>, Error>
    {
        use self::Route::*;
        assert!(end);
        let my_srv_id = self.wdata.runtime.server_id;
        let query = mem::replace(&mut self.state,
                                 State::Error(Status::InternalServerError));
        self.state = match query {
            State::Query(Subscribe(PubCid(cid, srv_id), topic)) => {
                if data.len() == 0 {
                    if srv_id == my_srv_id {
                        self.wdata.processor.send(Action::Subscribe {
                            conn_id: cid,
                            topic: topic.clone(),
                        });
                    } else {
                        debug!("Skipping action with non-local cid");
                    }
                    self.wdata.remote.send(RemoteAction::Subscribe {
                        conn_id: cid,
                        server_id: srv_id,
                        topic: topic,
                    });
                    State::Done
                } else {
                    State::Error(Status::BadRequest)
                }
            }
            State::Query(Unsubscribe(PubCid(cid, srv_id), topic)) => {
                if data.len() == 0 {
                    if srv_id == my_srv_id {
                        self.wdata.remote.send(RemoteAction::Unsubscribe {
                            conn_id: cid,
                            server_id: srv_id,
                            topic: topic.clone(),
                        });
                    } else {
                        debug!("Skipping action with non-local cid");
                    }
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
                match serde_json::from_slice(data) {
                    Ok(json) => {
                        // Send this Action to Replication Queue
                        let data: Arc<Json> = Arc::new(json);
                        self.wdata.remote.send(RemoteAction::Publish {
                            topic: topic.clone(),
                            data: data.clone(),
                        });
                        self.wdata.processor.send(Action::Publish {
                            topic: topic,
                            data: data,
                        });
                        State::Done
                    }
                    Err(e) => {
                        info!("Error decoding json for '/v1/publish': \
                            {:?}", e);
                        State::Error(Status::BadRequest)
                    }
                }
            }
            State::Query(LatticeSubscribe(PubCid(cid, srv_id), ns)) => {
                // TODO(tailhook) check content-type
                let data: Result<Delta,_> = serde_json::from_slice(data)
                    .map_err(|e| {
                        info!("Error decoding json for \
                            '/v1/lattice/_/subscribe': \
                            {:?}", e);
                    });
                match data {
                    Ok(delta) => {
                        self.wdata.remote.send(RemoteAction::Lattice {
                            namespace: ns.clone(),
                            delta: delta.clone(),
                        });
                        self.wdata.remote.send(RemoteAction::Attach {
                            namespace: ns.clone(),
                            conn_id: cid,
                            server_id: srv_id,
                        });
                        self.wdata.processor.send(Action::Lattice {
                            namespace: ns.clone(),
                            delta: delta,
                        });
                        if srv_id == my_srv_id {
                            self.wdata.processor.send(Action::Attach {
                                namespace: ns.clone(),
                                conn_id: cid,
                            });
                        } else {
                            debug!("Skipping action with non-local cid");
                        }
                        State::Done
                    }
                    Err(_) => {
                        State::Error(Status::BadRequest)
                    }
                }
            }
            State::Query(Detach(PubCid(cid, srv_id), ns)) => {
                self.wdata.remote.send(RemoteAction::Detach {
                    namespace: ns.clone(),
                    conn_id: cid.clone(),
                    server_id: srv_id,
                });
                if srv_id == my_srv_id {
                    self.wdata.processor.send(Action::Detach {
                        namespace: ns.clone(),
                        conn_id: cid,
                    });
                } else {
                    debug!("Skipping action with non-local cid");
                }
                State::Done
            }
            State::Query(Lattice(ns)) => {
                // TODO(tailhook) check content-type
                let data: Result<Delta,_> = serde_json::from_slice(data)
                    .map_err(|e| {
                        info!("Error decoding json for \
                            '/v1/lattice': \
                            {:?}", e);
                    });
                match data {
                    Ok(delta) => {
                        // Send this Action to Replication Queue
                        self.wdata.remote.send(RemoteAction::Lattice {
                            namespace: ns.clone(),
                            delta: delta.clone(),
                        });
                        self.wdata.processor.send(Action::Lattice {
                            namespace: ns.clone(),
                            delta: delta,
                        });
                        State::Done
                    }
                    Err(_) => {
                        State::Error(Status::BadRequest)
                    }
                }
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
            e.add_length(0).unwrap();
            e.done_headers().unwrap();
            ok(e.done())
        } else {
            e.status(Status::NoContent);
            e.done_headers().unwrap();
            ok(e.done())
        }
    }
}
