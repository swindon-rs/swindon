use std::io;
use std::sync::Arc;
use std::net::SocketAddr;

use futures::future::Either;
use futures::stream::Stream;
use tk_http;
use tk_http::server::Proto;
use futures::future::{Future, ok};
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Handle, Timeout};
use futures::sync::oneshot::{Receiver};

use intern::SessionPoolName;
use config::SessionPool;
use runtime::Runtime;
use chat::Shutdown;
use chat::listener::codec::Handler;
use chat::processor::ProcessorPool;


pub struct WorkerData {
    pub name: SessionPoolName,
    pub runtime: Arc<Runtime>,
    pub settings: Arc<SessionPool>,
    pub processor: ProcessorPool,
    pub handle: Handle, // Does it belong here?
}

pub fn listen(addr: SocketAddr, worker_data: &Arc<WorkerData>,
    shutter: Receiver<Shutdown>)
    -> Result<(), io::Error>
{
    let w1 = worker_data.clone();
    let w2 = worker_data.clone();
    let runtime = worker_data.runtime.clone();
    let h1 = runtime.handle.clone();
    let listener = TcpListener::bind(&addr, &worker_data.handle)?;
    // TODO(tailhook) how to update?
    let hcfg = tk_http::server::Config::new()
        .inflight_request_limit(worker_data.settings.pipeline_depth)
        // TODO(tailhook) make it configurable?
        .inflight_request_prealoc(0)
        .done();

    worker_data.handle.spawn(
        listener.incoming()
        // we need stream that doesn't fail on error
        .then(move |item| match item {
            Ok(x) => Either::A(ok(Some(x))),
            Err(e) => {
                warn!("Error accepting: {}", e);
                let dur = w1.settings.listen_error_timeout;
                Either::B(Timeout::new(*dur, &runtime.handle).unwrap()
                    .and_then(|()| ok(None)))
            }
        })
        .filter_map(|x| x)
        .map(move |(socket, saddr)| {
             Proto::new(socket, &hcfg, Handler::new(saddr, w2.clone()), &h1)
             // always succeed
             .then(|_| Ok(()))
        })
        .buffer_unordered(worker_data.settings.max_connections)
        .for_each(move |()| Ok(()))
        .select(shutter.then(move |_| Ok(())))
        .map(move |(_, _)| info!("Listener {} exited", addr))
        .map_err(move |(_, _)| info!("Listener {} exited", addr))
    );
    Ok(())
}
