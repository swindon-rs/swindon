use super::Frame;
use tk_bufstream::IoBuf;
use tokio_core::io::Io;

pub trait Dispatcher<S: Io> {
    /// Temporary solution is to output data directly
    fn dispatch(&mut self, frame: Frame, sock: &mut IoBuf<S>);
}
