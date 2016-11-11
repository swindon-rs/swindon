use std::sync::mpsc::Receiver;
use std::collections::HashMap;

use super::Event;
use super::pool::Pool;


pub fn run(rc: Receiver<Event>) {
    use super::Action::*;

    let mut pools = HashMap::new();

    for msg in rc.recv() {
        let Event { timestamp, action, pool } = msg;
        match action {

            // Pool management
            EnsureSessionPool(config) => {
                pools.insert(pool.clone(), Pool::new(pool, config));
            }
            StopSessionPool => {
                unimplemented!();
            }
            Cleanup => {
                pools.get_mut(&pool)
                .map(|p| p.cleanup(timestamp))
                .unwrap_or_else(|| debug!("Undefined pool {:?}", pool))
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
        }
    }
}
