use std::thread;
use std::time::Duration;

use async_slot as slot;

use crate::config::{Configurator};


fn updater(tx: slot::Sender<()>, mut configurator: Configurator) {
    loop {
        thread::sleep(Duration::new(10, 0));
        match configurator.try_update() {
            Ok(false) => {}
            Ok(true) => {
                tx.swap(()).expect("Can send updated config");
            }
            Err(e) => {
                error!("Reading new config: {}", e);
            }
        }
    }
}

pub fn update_thread(configurator: Configurator) -> slot::Receiver<()> {
    let (tx, rx) = slot::channel();
    thread::spawn(move || updater(tx, configurator));
    return rx;
}
