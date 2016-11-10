use std::sync::mpsc::Receiver;
use std::collections::HashMap;

use super::Event;
use super::pool::Pool;


pub fn run(rc: Receiver<Event>) {
    use super::Action::*;

    let mut pools = HashMap::new();

    for msg in rc.recv() {
        match msg.action {
            EnsureSessionPool(config) => {
                pools.insert(msg.pool, Pool::new(config));
            }
            StopSessionPool => {
                unimplemented!();
            }
        }
    }
}
