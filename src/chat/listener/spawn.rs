use std::io;
use std::sync::Arc;
use std::net::SocketAddr;

use futures::stream::Stream;
use minihttp;
use minihttp::server::Proto;
use futures::future::{Either, Future, ok};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Handle;
use futures::sync::oneshot::{channel as oneshot, Sender, Receiver};

use intern::SessionPoolName;
use config::SessionPool;
use runtime::Runtime;
use chat::listener::codec::Handler;


pub fn spawn_listener(addr: SocketAddr, handle: &Handle,
    runtime: &Arc<Runtime>, name: &SessionPoolName,
    settings: &Arc<SessionPool>, shutter: Receiver<()>)
    -> Result<(), io::Error>
{
    let root = runtime.config.get();
    let runtime = runtime.clone();
    let settings = settings.clone();
    let name = name.clone();
    let listener = TcpListener::bind(&addr, &handle)?;
    // TODO(tailhook) how to update?
    let mut hcfg = minihttp::server::Config::new()
        .inflight_request_limit(root.pipeline_depth)
        // TODO(tailhook) make it configurable?
        .inflight_request_prealoc(0)
        .done();
    let h1 = handle.clone();

    handle.spawn(
        listener.incoming()
        .then(move |item| match item {
            Ok((socket, saddr)) => {
                ok(Proto::new(socket, &hcfg,
                    Handler::new(runtime.clone(), name.clone(),
                                 settings.clone(), h1.clone())))
            }
            Err(e) => {
                info!("Error accepting: {}", e);
                unimplemented!();
                /*
                let dur = runtime.config.get().listen_error_timeout;
                Either::B(Timeout::new(*dur, &runtime.handle).unwrap()
                    .from_err()
                    .and_then(|()| Ok(())))
                */
            }
        })
        .buffer_unordered(root.max_connections)
        .for_each(|()| Ok(()))
        .select(shutter.map_err(|_| unreachable!()))
        .map(move |(_, _)| info!("Listener {} exited", addr))
        .map_err(move |(_, _)| info!("Listener {} exited", addr))
    );
    Ok(())
}
