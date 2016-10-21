use tokio_core::io::Io;
use tk_bufstream::IoBuf;

use super::{Dispatcher};
use super::Frame;


pub struct Echo;


impl<S: Io> Dispatcher<S> for Echo {
    fn dispatch(&mut self, frame: Frame, sock: &mut IoBuf<S>) {
        match frame {
            Frame::Ping(_) => {
                unimplemented!();
            }
            Frame::Pong(_) => { }  // track last ping?
            Frame::Text(x) => {
                println!("Received {:?}", x);
            }
            Frame::Binary(x) => {
                println!("Received (bin) {:?}", x);
            }
        }
    }
}

