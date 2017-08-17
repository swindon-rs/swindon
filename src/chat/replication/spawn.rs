use std::io;
use std::net::SocketAddr;
use std::time::Instant;
use std::sync::Arc;
use futures::{Future, Stream};
use futures::future::{FutureResult, Either, ok, err};
use futures::sync::oneshot::Receiver;
use futures::sync::mpsc::unbounded;
use tokio_core::reactor::{Timeout, Handle};
use tokio_core::net::{TcpListener, TcpStream};
use tk_listen::ListenExt;
use tk_http::server::{Proto, Config};
use tk_http::websocket::{Config as WsConfig, Loop};
use tk_http::websocket::{Dispatcher, Frame, Error};
use tk_http::websocket::client::HandshakeProto;
use abstract_ns::{Router, Resolver};
use serde_json;

use config::Replication;
use runtime::ServerId;
use super::server::Incoming;
use super::client::Authorizer;
use super::{IncomingChannel, ReplAction};
use chat::replication::{CONNECTIONS, FRAMES_SENT, FRAMES_RECEIVED};


pub fn listen(addr: SocketAddr, sender: IncomingChannel,
    server_id: &ServerId, settings: &Arc<Replication>,
    handle: &Handle, shutter: Receiver<()>)
    -> Result<(), io::Error>
{
    // TODO: setup proper configuration;
    let hcfg = Config::new().done();
    let h1 = handle.clone();
    let srv_id = server_id.clone();

    let listener = TcpListener::bind(&addr, &handle)?;
    handle.spawn(listener.incoming()
        .sleep_on_error(settings.listen_error_timeout, &handle)
        .map(move |(socket, _)| {
            let disp = Incoming::new(sender.clone(), srv_id, &h1);
            Proto::new(socket, &hcfg, disp, &h1)
            .map_err(|e| debug!("Http protocol error: {}", e))
        })
        .listen(settings.max_connections)
        .select(shutter.map_err(|_| unreachable!()))
        .map(move |(_, _)| info!("Listener {} exited", addr))
        .map_err(move |(_, _)| info!("Listener {} exited", addr))
    );
    Ok(())
}

pub fn connect(peer: &str, sender: IncomingChannel,
    server_id: &ServerId, timeout_at: Instant, handle: &Handle,
    resolver: &Router)
{
    let wcfg = WsConfig::new().done();
    let server_id = server_id.clone();
    let h1 = handle.clone();
    let h2 = handle.clone();
    let p1 = peer.to_string();
    let p2 = p1.clone();

    let timeout = Timeout::new_at(timeout_at, &handle)
    .expect("timeout created");

    handle.spawn(
    resolver.resolve(peer)
    .map_err(|e|
        // I'm not sure this is a good idea actually
        e.into_io())
    .and_then(|addr| {
        addr.pick_one().map_or(
            err(io::Error::new(io::ErrorKind::Other, "no address")),
            |a| ok(a))
    })
    .and_then(move |addr| TcpStream::connect(&addr, &h1))
    .select2(timeout)
    .then(|res| {
        match res {
            Ok(Either::A((stream, _))) => ok(stream),
            Ok(Either::B(((), _))) => err(format!("Connect timed out")),
            Err(Either::A((e, _))) |
            Err(Either::B((e, _))) => err(format!("Connect error: {}", e)),
        }
    })
    .and_then(move |sock| {
        HandshakeProto::new(sock, Authorizer::new(p1, server_id))
        .map_err(|e| format!("WS auth error: {}", e))
    })
    .and_then(move |(out, inp, remote_srv_id)| {
        let (tx, rx) = unbounded();
        let rx = rx
            .map_err(|_| format!("receiver error"))
            .map(|x| {
                FRAMES_SENT.incr(1);
                x
            });
        sender.send(ReplAction::Attach {
            tx: tx,
            server_id: remote_srv_id,
            peer: Some(p2),
        }).ok();
        CONNECTIONS.incr(1);
        Loop::client(out, inp, rx, Handler(sender), &wcfg, &h2)
        .map_err(|e| format!("WS loop error: {}", e))
    })
    .map_err(|e| error!("{}", e)));
}


pub struct Handler(pub IncomingChannel);

impl Dispatcher for Handler {
    type Future = FutureResult<(), Error>;

    fn frame (&mut self, frame: &Frame) -> Self::Future {
        if let &Frame::Text(data) = frame {
            FRAMES_RECEIVED.incr(1);
            match serde_json::from_str(data) {
                Ok(msg) => {
                    // TODO: make proper result handling
                    self.0.send(ReplAction::Incoming(msg)).ok();
                }
                Err(e) => {
                    return err(Error::custom(
                        format!("Error decoding message: {}", e)));
                }
            };
        }
        ok(())
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        CONNECTIONS.decr(1);
    }
}
