use std::io;
use std::str::{from_utf8, Utf8Error};

use netbuf::Buf;
use futures::{Future, Async, Poll};
use futures::Async::{Ready, NotReady};
use tokio_core::io::Io;
use tk_bufstream::IoBuf;
use byteorder::BigEndian;

use super::Dispatcher;
use self::Frame::*;

const MAX_MESSAGE_SIZE: u64 = 128 << 10;


quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: io::Error) {
            description("IO error")
            display("IO error: {}", err)
            from()
        }
        InvalidUtf8(err: Utf8Error) {
            description("Error decoding text frame")
            display("Error decoding text frame: {}", err)
            from()
        }
        InvalidOpcode(code: u8) {
            description("Opcode of the frame is invalid")
            display("Opcode of the frame is invalid: {}", code)
            from()
        }
        Unmasked {
            description("Received unmasked frame")
        }
        TooLong {
            description("Received frame that is too long")
        }
    }
}


pub struct WebsockProto<S: Io, D: Dispatcher<S>> {
    dispatcher: D,
    io: IoBuf<S>,
}

pub enum Frame<'a> {
    Ping(&'a [u8]),
    Pong(&'a [u8]),
    Text(&'a str),
    Binary(&'a [u8]),
}

fn parse_frame<'x>(buf: &'x mut Buf) -> Poll<(Frame<'x>, usize), Error> {
    if buf.len() < 2 {
        return Ok(NotReady);
    }
    let (size, fsize) = {
        match buf[1] & 0x7F {
            126 => {
                if buf.len() < 4 {
                    return Ok(NotReady);
                }
                (BigEndian.read_u16(buf[2..4]) as u64, 4)
            }
            127 => {
                if buf.len() < 10 {
                    return Ok(NotReady);
                }
                (BigEndian::read_u64(buf[2..10]), 10)
            }
            size => (size as u64, 1),
        }
    };
    if size > MAX_MESSAGE_SIZE {
        return Err(Error::TooLong);
    }
    let size = size as usize;
    let start = fsize + 4 /* mask size */;
    if buf.len() < start + size {
        return Ok(NotReady);
    }

    let fin = buf[0] & 0x80 != 0;
    let opcode = buf[0] & 0x0F;
    // TODO(tailhook) should we assert that reserved bits are zero?
    let mask = buf[1] & 0x80 != 0;
    if !fin {
        unimplemented!();  // framed(chunked) messages
    }
    if !mask {
        return Err(Error::Unmasked);
    }
    let mask: [u8; 4] = buf[start-4..start];
    for idx in 0..size { // hopefully llvm is smart enough to optimize it
        buf[start + idx] |= mask[idx % 4];
    }
    let data = &buf[start..(start + size)];
    let frame = match opcode {
        0x9 => Ping(data),
        0xA => Pong(data),
        0x1 => Text(try!(from_utf8(data))),
        0x2 => Binary(data),
        x => return Err(Error::InvalidOpcode(x)),
    };
    return (frame, start + size);
}


impl<D, S: Io> Future for WebsockProto<S, D>
    where D: Dispatcher<S>,
{
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<(), Error> {
        loop {
            try!(self.io.flush());
            if let Some((frame, bytes)) = parse_frame(self.io.in_buf) {
                try!(self.dispatcher.dispatch(frame, self.io));
                self.io.in_buf.consume(bytes);  // consume together with '\n'
            } else {
                let nbytes = try!(self.io.read());
                if nbytes == 0 {
                    if self.io.done() {
                        return Ok(Async::Ready(()));
                    } else {
                        return Ok(Async::NotReady);
                    }
                }
            }
        }
    }
}
