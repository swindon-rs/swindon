use std::sync::Arc;

use futures::Future;
use futures::stream::{Stream};
use futures::{AsyncSink, Sink};
use futures::sync::oneshot::{channel as oneshot, Sender};
use futures::sync::mpsc::{UnboundedReceiver as Receiver};
use tokio_core::reactor::Handle;

use http_pools::{REQUESTS, FAILED_503};
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
        let suffix = if settings.use_tangle_prefix.unwrap_or(false) {
            "/tangle/session_inactive"
        } else {
            "/swindon/session_inactive"
        };
        // TODO(tailhook) these arcs can be created at config read
        let path = if dest.path == "/" {
            Arc::new(suffix.to_string())
        } else {
            Arc::new(dest.path.to_string() + suffix)
        };
        handlers.push((path, dest.upstream.clone()));
    }
    let settings = settings.clone();
    let (tx, rx) = oneshot();
    handle.spawn(stream.for_each(move |msg| {
            match msg {
                InactiveSession { session_id, .. } => {
                    info!("Sending inactivity: {:?}", session_id);
                    for &(ref path, ref upname) in &handlers {
                        let config = runtime.config.get();
                        let dest_settings = match
                            config.http_destinations.get(upname)
                        {
                            Some(x) => x,
                            None => {
                                error!("No such destination {:?} \
                                    for sending inactivity",
                                    upname);
                                continue;
                            }
                        };
                        let mut up = runtime.http_pools.upstream(&upname);
                        let codec = Box::new(backend::InactivityCodec::new(
                            path, &session_id, dest_settings,
                            settings.use_tangle_auth.unwrap_or(false)));
                        match up.get_mut().get_mut() {
                            Some(pool) => {
                                match pool.start_send(codec) {
                                    Ok(AsyncSink::NotReady(_)) => {
                                        FAILED_503.incr(1);
                                        // TODO(tailhook) retry later
                                        warn!("Coudn't send inactivity");
                                    }
                                    Ok(AsyncSink::Ready) => {
                                        REQUESTS.incr(1);
                                        debug!("Sent /swindon/session_inactive");
                                    }
                                    Err(e) => {
                                        // TODO(tailhook) log, retry later
                                        error!("Error sending inactivity: {}",
                                            e);
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
