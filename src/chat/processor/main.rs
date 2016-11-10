use std::sync::mpsc::Receiver;
use std::collections::HashMap;

use super::Event;
use super::pool::Pool;


pub fn run(rc: Receiver<Event>) {
    use super::Action::*;

    let mut pools = HashMap::new();

    for msg in rc.recv() {
        match msg.action {

            // Pool management
            EnsureSessionPool(config) => {
                pools.insert(msg.pool.clone(), Pool::new(msg.pool, config));
            }
            StopSessionPool => {
                unimplemented!();
            }

            // Connection management
            NewConnection { user_id, conn_id, metadata } => {
                let pool = msg.pool;
                pools.get_mut(&pool)
                .map(|p| p.add_connection(user_id, conn_id, metadata))
                .unwrap_or_else(|| debug!("Undefined pool {:?}", pool))
            }
        }
    }
}
