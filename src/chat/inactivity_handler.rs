use std::sync::Arc;
use std::borrow::Cow;

use futures::Future;
use futures::stream::{Stream};
use futures::{AsyncSink, Sink};
use futures::sync::oneshot::{channel as oneshot, Sender};
use futures::sync::mpsc::{UnboundedReceiver as Receiver};
use tokio_core::reactor::Handle;

use runtime::Runtime;
use config::SessionPool;
use chat::Shutdown;
use chat::backend;
use chat::processor::{PoolMessage};


pub fn run(runtime: &Arc<Runtime>, settings: &Arc<SessionPool>,
           handle: &Handle, stream: Receiver<PoolMessage>)
    -> Sender<Shutdown>
{
    use chat::processor::PoolMessage::*;
    let mut handlers = Vec::new();
    let runtime = runtime.clone();
    for dest in &settings.inactivity_handlers {
        let path = if dest.path == "/" {
            Arc::new("/tangle/session_inactive".to_string())
        } else {
            Arc::new(dest.path.to_string() + "/tangle/session_inactive")
        };
        handlers.push((path, dest.upstream.clone()));
    }
    let (tx, rx) = oneshot();
    handle.spawn(stream.for_each(move |msg| {
            match msg {
                InactiveSession { session_id, .. } => {
                    info!("Sending inactivity: {:?}", session_id);
                    for &(ref path, ref upname) in &handlers {
                        let mut up = runtime.http_pools.upstream(&upname);
                        let codec = Box::new(backend::InactivityCodec::new(
                            path, &session_id));
                        match up.get_mut().get_mut() {
                            Some(pool) => {
                                match pool.start_send(codec) {
                                    Ok(AsyncSink::NotReady(_)) => {
                                        // TODO(tailhook) retry later
                                        warn!("Coudn't send inactivity");
                                    }
                                    Ok(AsyncSink::Ready) => {
                                        debug!("Sent /tangle/session_inactive");
                                    }
                                    Err(e) => {
                                        // TODO(tailhook) log, retry later
                                    }
                                }
                            }
                            None => {
                                error!("No such destination {:?} \
                                    for sending inactivity",
                                    upname);
                            }
                        }
                    }
                }
            }
            Ok(())
        })
        .select(rx.then(move |_| Ok(())))
        .map(move |(_, _)| info!("Inactivity handler exited"))
        .map_err(move |(_, _)| info!("Inactivity handler exited")));
    return tx;
}
