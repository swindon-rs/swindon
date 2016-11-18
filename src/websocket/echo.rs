use std::time::Duration;

use futures::Future;
use tokio_core::reactor::{Handle, Timeout};

use super::{Dispatcher, Frame, Error, ImmediateReplier, RemoteReplier};


pub struct Echo(pub Handle, pub RemoteReplier);


impl Dispatcher for Echo {
    fn dispatch(&mut self, frame: Frame, replier: &mut ImmediateReplier)
        -> Result<(), Error>
    {
        match frame {
            Frame::Ping(data) => {
                debug!("Got ping");
                replier.pong(data);
            }
            Frame::Pong(_) => { }  // track last ping?
            Frame::Text(data) => {
                if data.starts_with("alarm ") {
                    if let Ok(num) = data[6..].parse() {
                        let remote = self.1.clone();
                        let timeout = Timeout::new(Duration::new(num, 0),
                                                   &self.0)
                            .expect("can set timeout")
                            // TODO(tailhook) another misleading error type
                            .map_err(|e| e.into())
                            .and_then(move |()| {
                                remote.send_text("Alarm triggered")
                            })
                            .map_err(|e| info!("Alarm error: {}", e));
                        self.0.spawn(timeout);
                        replier.text("Okay, the alarm is set");
                        return Ok(())
                    }
                }
                debug!("Echoing {:?}", data);
                replier.text(data);
            }
            Frame::Binary(data) => {
                debug!("Echoing (bin) {:?}", String::from_utf8_lossy(data));
                replier.binary(data);
            }
        }
        Ok(())
    }
}
