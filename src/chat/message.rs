/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::io;
use std::str::{self, Utf8Error};
use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json, ParserError};
use rustc_serialize::{Encodable, Encoder};

use minihttp::enums::Status;
use intern::SessionId;

pub type Meta = BTreeMap<String, Json>;
pub type Args = Vec<Json>;
pub type Kwargs = BTreeMap<String, Json>;

pub enum Message {
    /// Websocket call.
    Call(String, Args, Kwargs),
    /// Auth message
    Auth(Kwargs),
    /// Session inactive message.
    Inactive,

    // Backend messages

    /// Message::Call result
    Result(Json),
    /// Auth result
    Hello(SessionId, Json),
    /// Backend "topic publish" message
    Message(Json),
    /// Lattice update Kind
    Lattice(Json),
    /// Error
    Error(MessageError),
}


impl Message {

    /// Message request method
    pub fn method(&self) -> &str {
        match *self {
            Message::Call(ref m, _, _) => m.as_str(),
            Message::Auth(_) => "tangle.authorize_connection",
            Message::Inactive => "tangle.inactive",
            _ => unreachable!()
        }
    }

    /// Encode message with meta to JSON.
    pub fn encode_with(&self, meta: &Meta) -> String {
        json::encode(&Payload(Some(meta), self)).unwrap()
    }

    /// Encodes message to String with empty meta.
    pub fn encode(&self) -> String {
        json::encode(&Payload(None, self)).unwrap()
    }

    /// Adds extra fields to Meta object depending on self type.
    pub fn update_meta(&self, meta: &mut Meta) {
        use self::MessageError::*;
        match *self {
            Message::Error(HttpError(status, _)) => {
                meta.insert(
                    "error_kind".into(), Json::String("http_error".into()));
                meta.insert(
                    "status".into(), Json::U64(status.code() as u64));
            }
            Message::Error(_) => {
                meta.insert("error_kind".into(),
                    Json::String("invalid_content_type".into()));
            }
            _ => {}
        }
    }
}

/// Decode Websocket json message into Meta & Message structs.
pub fn decode_message(s: &str)
    -> Result<(Meta, Message), MessageError>
{
    // TODO: replace MessageError here with ProtocolError
    //      ProtocolError can't be sent back.
    use self::ValidationError::*;
    let invalid_method = |m: &str| {
        m.starts_with("tangle.") | m.find("/").is_some()
    };
    match Json::from_str(s) {
        Ok(Json::Array(mut message)) => {
            use rustc_serialize::json::Json::*;

            if message.len() != 4 {
                return Err(InvalidLength.into())
            }
            let kwargs = match message.pop() {
                Some(Object(kwargs)) => kwargs,
                _ => return Err(ObjectExpected.into()),
            };
            let args = match message.pop() {
                Some(Array(args)) => args,
                _ => return Err(ArrayExpected.into()),
            };
            let meta = match message.pop() {
                Some(Object(meta)) => meta,
                _ => return Err(ObjectExpected.into()),
            };
            match meta.get("request_id".into()) {
                Some(&Json::String(_)) |
                    Some(&Json::I64(_)) |
                    Some(&Json::U64(_)) |
                    Some(&Json::F64(_)) => {},
                _ => return Err(InvalidRequestId.into()),
            };
            let method = match message.pop() {
                Some(Json::String(method)) => {
                    if invalid_method(&method) {
                        return Err(InvalidMethod.into());
                    }
                    method
                }
                _ => return Err(InvalidMethod.into()),
            };
            Ok((meta, Message::Call(method, args, kwargs)))
        }
        Ok(_) => Err(ArrayExpected.into()),
        Err(e) => Err(e.into())
    }
}


#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// Invalid message length;
    InvalidLength,
    /// Invalid method ("tangle." or contains ".");
    InvalidMethod,
    /// request_id is missing or invalid in request_meta object;
    InvalidRequestId,
    /// user_id is missing or invalid in request_meta object;
    InvalidUserId,
    /// Array of args expected;
    ArrayExpected,
    /// Meta/Kwargs object expected;
    ObjectExpected,
}

quick_error! {
    #[derive(Debug)]
    pub enum MessageError {
        IoError(err: io::Error) {
            description(err.description())
            display("I/O error: {:?}", err)
            from()
        }
        /// Message validation error;
        ValidationError(err: ValidationError) {
            description("Message validation error")
            display("Validation error: {:?}", err)
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
        /// Response Http Error;
        HttpError(status: Status, body: Option<Json>) {
            // from()
            description("Http error")
            display("Http error: {}: {:?}", status.code(), body)
        }
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
        }
    }
}


struct Payload<'a>(Option<&'a Meta>, &'a Message);

impl<'a> Encodable for Payload<'a> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        use self::Message::*;

        s.emit_seq(3, |s| {
            match self.1 {
                &Call(_, ref args, ref kwargs) => {
                    s.emit_seq_elt(0, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(1, |s| args.encode(s))?;
                    s.emit_seq_elt(2, |s| kwargs.encode(s))?;
                }
                &Auth(ref kwargs) => {
                    s.emit_seq_elt(0, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(1, |s| s.emit_seq(0, |_| Ok(())) )?;
                    s.emit_seq_elt(2, |s| kwargs.encode(s))?;
                }
                &Inactive => {
                    s.emit_seq_elt(0, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(1, |s| s.emit_seq(0, |_| Ok(())) )?;
                    s.emit_seq_elt(2, |s| s.emit_map(0, |_| Ok(())) )?;
                }
                &Result(ref value) => {
                    s.emit_seq_elt(0, |s| s.emit_str("result"))?;
                    s.emit_seq_elt(1, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(2, |s| value.encode(s))?;
                }
                &Hello(_, ref value) => {
                    s.emit_seq_elt(0, |s| s.emit_str("result"))?;
                    s.emit_seq_elt(1, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(2, |s| value.encode(s))?;
                }
                &Message(ref value) => {
                    s.emit_seq_elt(0, |s| s.emit_str("message"))?;
                    s.emit_seq_elt(1, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(2, |s| value.encode(s))?;
                }
                &Lattice(ref value) => {
                    s.emit_seq_elt(0, |s| s.emit_str("lattice"))?;
                    s.emit_seq_elt(1, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(2, |s| value.encode(s))?;
                }
                &Error(ref value) => {
                    s.emit_seq_elt(0, |s| s.emit_str("error"))?;
                    s.emit_seq_elt(1, |s| self.encode_meta(s))?;
                    s.emit_seq_elt(2, |s| value.encode(s))?;
                }
            }
            Ok(())
        })
    }
}

impl<'a> Payload<'a> {
    fn encode_meta<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match self.0 {
            Some(meta) => meta.encode(s),
            None => s.emit_map(0, |_| Ok(())),
        }
    }
}

#[cfg(test)]
mod test {
    // TODO: REWRITE TESTS;

    // use rustc_serialize::json::{Json, ParserError, ErrorCode};
    // use minihttp::enums::Status;
    // use super::*;

    // fn default() -> (Meta, Message) {
    //     decode_message(
    //         "[\"hello\", {\"request_id\": \"req123\"}, [], {}]")
    //         .unwrap()
    // }

    // #[test]
    // fn decode() {
    //     let msg = decode_message("[]");
    //     assert_eq!(msg.err().unwrap(), MessageError::InvalidLength);

    //     let msg = decode_message(
    //         "[\"hello\", {\"request_id\": \"123\"}, [], {}]");
    //     assert!(msg.is_ok());
    //     let (meta, msg) = msg.unwrap();
    //     assert_eq!(msg.method, "hello".to_string());
    //     assert_eq!(msg.request_id, "123".to_string());
    //     assert_eq!(msg.active, None);
    //     assert_eq!(msg.args.len(), 0);
    //     assert_eq!(msg.kwargs.len(), 0);
    // }

    // #[test]
    // fn decode_message_errors() {
    //     let err = decode_message(
    //         "[\"bad/method\", {\"request_id\": \"123\"}, [], {}]")
    //         .err().unwrap();
    //     assert_eq!(err, MessageError::InvalidMethod);

    //     let err = decode_message(
    //         "[\"good.method\", {\"request_id\": \"123\"}, []]")
    //         .err().unwrap();
    //     assert_eq!(err, MessageError::InvalidLength);
    // }

    // #[test]
    // fn method_func() {
    //     let message = default();

    //     assert_eq!(message.method(), "hello");
    // }

    // #[test]
    // fn payload() {
    //     let mut message = default();
    //     message.meta.insert("debug".into(), Json::Boolean(true));

    //     let result = message.payload();
    //     assert_eq!(result, "[{\"request_id\":\"req123\",\"debug\":true},\
    //         [],{}]".to_string());

    //     message.args.push(Json::String("msg".to_string()));

    //     let result = message.payload();
    //     assert_eq!(result, "[{\"request_id\":\"req123\",\"debug\":true},\
    //         [\"msg\"],{}]".to_string());

    //     message.kwargs.insert("name".into(), Json::String("test".to_string()));
    //     let result = message.payload();
    //     assert_eq!(result, "[{\"request_id\":\"req123\",\"debug\":true},\
    //         [\"msg\"],{\"name\":\"test\"}]".to_string());
    // }

    // #[test]
    // fn encode_result() {
    //     let result = Message::Result(
    //         Json::from_str("{\"result\": null}").unwrap());
    //     let meta = Meta::new();

    //     let result = result.encode_with(&meta);
    //     assert_eq!(result, "[\"result\",{\"request_id\":\"req123\"},\
    //         {\"result\":null}]");
    // }

    // #[test]
    // fn encode_error() {
    //     let (mut meta, _) = default();
    //     meta.insert("debug".into(), Json::Boolean(true));
    //     meta.insert("error_kind".into(), Json::Boolean(true));
    //     meta.insert("status".into(), Json::Boolean(false));

    //     let err = MessageError::HttpError(Status::BadRequest, None);
    //     let message = Message::Error(err);
    //     let result = message.encode_with(&meta);
    //     assert_eq!(result, "[\"error\",\
    //         {\"request_id\":\"req123\",\
    //         \"error_kind\":\"http_error\",\"status\":400,\
    //         \"debug\":true},null]");

    //     let json = Json::from_str("{\"message\":\"data required\"}").unwrap();
    //     let err = MessageError::HttpError(Status::BadRequest, Some(json));
    //     let message = Message::Error(err);
    //     let result = message.encode_with(&meta);
    //     assert_eq!(result, "[\"error\",{\"request_id\":\"req123\",\
    //         \"error_kind\":\"http_error\",\"status\":400,\"debug\":true},\
    //         {\"message\":\"data required\"}]");

    //     let err = MessageError::JsonError(
    //         ParserError::SyntaxError(ErrorCode::InvalidSyntax, 0, 0));
    //     let message = Message::Error(err);
    //     let result = message.encode_with(&meta);
    //     assert_eq!(result, "[\"error\",{\"request_id\":\"req123\",\
    //         \"error_kind\":\"invalid_content_type\",\
    //         \"debug\":true,\"status\":false},\
    //         \"SyntaxError(\\\"invalid syntax\\\", 0, 0)\"]");
    // }
}
