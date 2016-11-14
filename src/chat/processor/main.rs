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
        EnsureSessionPool(_) => unreachable!(),
        StopSessionPool => unreachable!(),
        // Connection management
        NewConnection { user_id, conn_id, metadata } => {
            pool.add_connection(ts, user_id, conn_id, metadata);
        }
        UpdateActivity { user_id, timestamp } => {
            pool.update_activity(user_id, timestamp);
        }
        Disconnect { user_id, conn_id } => {
            pool.del_connection(user_id, conn_id);
        }
    }
}

pub fn run(rx: Receiver<Event>) {
    use super::Action::*;
    use std::sync::mpsc::RecvTimeoutError::*;

    let mut pools = HashMap::new();

    loop {
        let timeout = pools.iter_mut()
            .map(|(_, pool): (_, &mut Pool)| pool.cleanup(Instant::now()))
            .flat_map(|x| x)
            .min();
        let result = match timeout {
            Some(t) => rx.recv_timeout(t.duration_since(Instant::now())),
            None => rx.recv().map_err(|_| Disconnected),
        };
        let value = match result {
            Ok(x) => Some(x),
            Err(Timeout) => continue,
            Err(Disconnected) => {
                panic!("Process pools is not needed for anyone");
            }
        };
        for msg in value.into_iter().chain(try_iter(&rx)) {
            let Event { timestamp, action, pool } = msg;
            match action {
                // Pool management
                EnsureSessionPool(config) => {
                    pools.insert(pool.clone(), Pool::new(pool, config));
                }
                StopSessionPool => {
                    unimplemented!();
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
