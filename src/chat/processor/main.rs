use std::time::Instant;
use std::sync::mpsc::Receiver;
use std::collections::HashMap;

use super::Event;
use super::pool::Pool;
use super::try_iter::try_iter;


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

                // Connection management
                NewConnection { user_id, conn_id, metadata } => {
                    pools.get_mut(&pool)
                    .map(|p| p.add_connection(timestamp,
                                              user_id, conn_id, metadata))
                    .unwrap_or_else(|| debug!("Undefined pool {:?}", pool))
                }
                UpdateActivity { user_id, timestamp } => {
                    pools.get_mut(&pool)
                    .map(|p| p.update_activity(user_id, timestamp))
                    .unwrap_or_else(|| debug!("Undefined pool {:?}", pool))
                }
                Disconnect { user_id, conn_id } => {
                    pools.get_mut(&pool)
                    .map(|p| p.del_connection(user_id, conn_id))
                    .unwrap_or_else(|| debug!("Undefined pool {:?}", pool))
                }
            }
        }
    }
}
