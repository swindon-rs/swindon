use super::{Dispatcher, Frame, Error, ImmediateReplier};


pub struct Echo;


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
