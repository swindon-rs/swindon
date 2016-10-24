use tk_bufstream::{Buf};

use super::{Frame, Error};
use websocket::write::WriteExt;


pub trait Dispatcher {
    /// Temporary solution is to output data directly
    fn dispatch(&mut self, frame: Frame, sock: &mut ImmediateReplier)
        -> Result<(), Error>;
}

pub struct ImmediateReplier<'a>(&'a mut Buf);

impl<'a> ImmediateReplier<'a> {
    pub fn new(buf: &'a mut Buf) -> ImmediateReplier<'a> {
        ImmediateReplier(buf)
    }
    pub fn pong(&mut self, data: &[u8]) {
        self.0.write_packet(0xA, data);
    }
    pub fn text(&mut self, data: &str) {
        self.0.write_packet(0x1, data.as_bytes());
    }
    pub fn binary(&mut self, data: &[u8]) {
        self.0.write_packet(0x2, data);
    }
}
