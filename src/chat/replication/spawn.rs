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
use rustc_serialize::json;

use config::Replication;
use runtime::RuntimeId;
use super::server::Incoming;
use super::client::Authorizer;
use super::{IncomingChannel, ReplAction};


pub fn listen(addr: SocketAddr, sender: IncomingChannel,
    runtime_id: &RuntimeId, settings: &Arc<Replication>,
    handle: &Handle, shutter: Receiver<()>)
    -> Result<(), io::Error>
{
    // TODO: setup proper configuration;
    let hcfg = Config::new().done();
    let h1 = handle.clone();
    let rid = runtime_id.clone();

    let listener = TcpListener::bind(&addr, &handle)?;
    handle.spawn(listener.incoming()
        .sleep_on_error(*settings.listen_error_timeout, &handle)
        .map(move |(socket, saddr)| {
            let disp = Incoming::new(saddr, sender.clone(), rid, &h1);
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

pub fn connect(addr: SocketAddr, sender: IncomingChannel,
    runtime_id: &RuntimeId, timeout_at: Instant, handle: &Handle)
{
    let wcfg = WsConfig::new().done();
    let runtime_id = runtime_id.clone();

    let timeout = Timeout::new_at(timeout_at, &handle)
    .expect("timeout created");

    handle.spawn(TcpStream::connect(&addr, &handle)
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
        HandshakeProto::new(sock, Authorizer::new(addr, runtime_id))
        .map_err(|e| format!("WS auth error: {}", e))
    })
    .and_then(move |(out, inp, (addr, runtime_id))| {
        let (tx, rx) = unbounded();
        let rx = rx.map_err(|_| format!("receiver error"));
        sender.send(ReplAction::Attach {
            tx: tx,
            runtime_id: runtime_id,
            addr: addr,
        });
        Loop::client(out, inp, rx, Handler(sender), &wcfg)
        .map_err(|e| format!("WS loop error: {}", e))
    })
    .map_err(|e| error!("{}", e)));
}


pub struct Handler(pub IncomingChannel);

impl Dispatcher for Handler {
    type Future = FutureResult<(), Error>;

    fn frame (&mut self, frame: &Frame) -> Self::Future {
        if let &Frame::Text(data) = frame {
            match json::decode(data) {
                Ok(action) => {
                    debug!("Received action: {:?}", action);
                    self.0.send(action);
                }
                Err(e) => {
                    error!("Error decoding message: {}", e);
                    return err(Error::custom(
                        format!("Error decoding message: {}", e)));
                }
            };
        }
        ok(())
    }
}
