use std::time::Instant;
use std::sync::mpsc::Receiver;
use std::collections::HashMap;

use super::{Event, Action};
use super::pool::Pool;
use super::try_iter::try_iter;

fn pool_action(pool: &mut Pool, ts: Instant, action: Action) {
    use super::Action::*;
    match action {
        // handled earlier
        NewSessionPool {..} => unreachable!(),
        StopSessionPool => unreachable!(),
        // Connection management
        NewConnection { conn_id, channel } => {
            pool.add_connection(conn_id, channel);
        }
        Associate { session_id, conn_id, metadata } => {
            pool.associate(conn_id, session_id, ts, metadata);
        }
        UpdateActivity { conn_id, timestamp } => {
            pool.update_activity(conn_id, timestamp);
        }
        Disconnect { conn_id } => {
            pool.del_connection(conn_id);
        }
        // Subscriptions
        Subscribe { conn_id, topic } => {
            pool.subscribe(conn_id, topic);
        }
        Unsubscribe { conn_id, topic } => {
            pool.unsubscribe(conn_id, topic);
        }
        Publish { topic, data } => {
            pool.publish(topic, data);
        }
        // Lattices
        Attach { conn_id, namespace } => {
            pool.lattice_attach(conn_id, namespace);
        }
        Lattice { namespace, delta } => {
            pool.lattice_update(namespace, delta);
        }
        Detach { conn_id, namespace } => {
            pool.lattice_detach(conn_id, namespace);
        }
    }
}

pub fn run(rx: Receiver<Event>) {
    use super::Action::*;
    use std::sync::mpsc::RecvTimeoutError::*;

    let mut pools = HashMap::new();

    loop {
        let now = Instant::now();
        let timeout = pools.iter_mut()
            .map(|(_, pool): (_, &mut Pool)| pool.cleanup(now))
            .flat_map(|x| x)
            .min();
        let result = match timeout {
            Some(t) => rx.recv_timeout(t.duration_since(now)),
            None => rx.recv().map_err(|_| Disconnected),
        };
        let value = match result {
            Ok(x) => Some(x),
            Err(Timeout) => continue,
            Err(Disconnected) => {
                panic!("Process pools are not needed for anyone");
            }
        };
        for msg in value.into_iter().chain(try_iter(&rx)) {
            let Event { timestamp, action, pool } = msg;
            debug!("Received action {:?} {:?}", pool, action);
            match action {
                // Pool management
                NewSessionPool { config, channel } => {
                    pools.insert(pool.clone(),
                        Pool::new(pool, config, channel));
                }
                StopSessionPool => {
                    if let Some(pool) = pools.remove(&pool) {
                        pool.stop();
                    }
                }
                _ => {
                    // For all other actions we resolve pool first
                    pools.get_mut(&pool)
                    .map(|p| pool_action(p, timestamp, action))
                    .unwrap_or_else(|| debug!("Undefined pool {:?}", pool))
                }
            }
        }
    }
}
