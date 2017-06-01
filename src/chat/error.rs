use std::io;
use std::str::Utf8Error;
use futures::sync::mpsc::SendError;

use serde::ser::{Serialize, Serializer};
use serde_json::{Error as JsonError, Value};
use tk_http::Status;
use tk_http::client;

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
        JsonError(err: JsonError) {
            description(err.description())
            display("JSON error: {}", err)
            from()
        }
        /// Protocol Message validation error;
        ValidationError(reason: String) {
            description("Message validation error")
            display("Validation error: {}", reason)
            from()
        }
        /// Backend application Error;
        HttpError(status: Status, body: Option<Value>) {
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

impl Serialize for MessageError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use self::MessageError::*;
        match *self {
            HttpError(_, None) => {
                serializer.serialize_none()
            }
            HttpError(_, Some(ref j)) => {
                j.serialize(serializer)
            }
            Utf8Error(ref err) => {
                serializer.serialize_str(format!("{}", err).as_str())
            }
            JsonError(ref err) => {
                serializer.serialize_str(format!("{}", err).as_str())
            }
            ValidationError(ref err) => {
                serializer.serialize_str(format!("{:?}", err).as_str())
            }
            IoError(ref err) => {
                serializer.serialize_str(format!("{:?}", err).as_str())
            }
            Proto(_) => {
                serializer.serialize_str("backend_protocol_error")
            }
            PoolOverflow => {
                serializer.serialize_str("too_many_requests")
            }
            PoolError => {
                serializer.serialize_str("unexpected_pool_error")
            }
        }
    }
}
