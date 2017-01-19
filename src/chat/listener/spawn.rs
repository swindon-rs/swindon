use std::io;
use std::sync::Arc;
use std::net::SocketAddr;

use futures::stream::Stream;
use minihttp;
use minihttp::server::Proto;
use futures::future::{Future, ok};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Handle;
use futures::sync::oneshot::{Sender, Receiver};

use intern::SessionPoolName;
use config::SessionPool;
use runtime::Runtime;
use chat::listener::codec::Handler;
use chat::processor::ProcessorPool;


pub struct WorkerData {
    pub name: SessionPoolName,
    pub runtime: Arc<Runtime>,
    pub settings: Arc<SessionPool>,
    pub processor: ProcessorPool,
    pub handle: Handle, // Does it belong here?
}

pub struct Shutdown;


pub fn listen(addr: SocketAddr, worker_data: &Arc<WorkerData>,
    shutter: Receiver<Shutdown>)
    -> Result<(), io::Error>
{
    let root = worker_data.runtime.config.get();
    let w1 = worker_data.clone();
    let listener = TcpListener::bind(&addr, &worker_data.handle)?;
    // TODO(tailhook) how to update?
    let hcfg = minihttp::server::Config::new()
        .inflight_request_limit(root.pipeline_depth)
        // TODO(tailhook) make it configurable?
        .inflight_request_prealoc(0)
        .done();

    worker_data.handle.spawn(
        listener.incoming()
        .then(move |item| match item {
            Ok((socket, saddr)) => {
                ok(Proto::new(socket, &hcfg,
                    Handler::new(saddr, w1.clone())))
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
        .for_each(move |()| Ok(()))
        .select(shutter.then(move |_| Ok(())))
        .map(move |(_, _)| info!("Listener {} exited", addr))
        .map_err(move |(_, _)| info!("Listener {} exited", addr))
    );
    Ok(())
}
