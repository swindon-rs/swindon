use std::io;
use std::str::Utf8Error;
use futures::sync::mpsc::SendError;

use rustc_serialize::{Encodable, Encoder};
use rustc_serialize::json::{Json, ParserError};
use tk_http::Status;
use tk_http::client;

use super::message::{ValidationError};

quick_error! {
    #[derive(Debug)]
    pub enum MessageError {
        /// Http client request error;
        IoError(err: io::Error) {
            description(err.description())
            display("I/O error: {:?}", err)
            from()
        }
        /// Utf8 decoding error;
        Utf8Error(err: Utf8Error) {
            description(err.description())
            display("Decode error {}", err)
            from()
        }
        /// JSON Parser Error;
        JsonError(err: ParserError) {
            description(err.description())
            display("JSON error: {}", err)
            from()
        }
        /// Protocol Message validation error;
        ValidationError(err: ValidationError) {
            description("Message validation error")
            display("Validation error: {:?}", err)
            from()
        }
        /// Backend application Error;
        HttpError(status: Status, body: Option<Json>) {
            // from()
            description("Http error")
            display("Http error: {}: {:?}", status.code(), body)
        }
        /// Coundn't send request by HTTP, got network or protocol error
        Proto(err: client::Error) {
            from()
            description("Http error")
            display("Http error: {}", err)
        }
        /// Too many requests queued
        PoolOverflow {
            description("too many requests queued")
        }
        /// Error sending message to worker pool
        PoolError {
            description("error sending message to worker pool")
        }
    }
}

impl<T> From<SendError<T>> for MessageError {
    fn from(_: SendError<T>) -> MessageError {
        MessageError::PoolError
    }
}

impl Encodable for MessageError {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        use self::MessageError::*;
        match *self {
            HttpError(_, None) => {
                s.emit_nil()
            }
            HttpError(_, Some(ref j)) => {
                j.encode(s)
            }
            Utf8Error(ref err) => {
                s.emit_str(format!("{}", err).as_str())
            }
            JsonError(ref err) => {
                s.emit_str(format!("{}", err).as_str())
            }
            ValidationError(ref err) => {
                s.emit_str(format!("{:?}", err).as_str())
            }
            IoError(ref err) => {
                s.emit_str(format!("{:?}", err).as_str())
            }
            Proto(_) => {
                s.emit_str("backend_protocol_error")
            }
            PoolOverflow => {
                s.emit_str("too_many_requests")
            }
            PoolError => {
                s.emit_str("unexpected_pool_error")
            }
        }
    }
}
