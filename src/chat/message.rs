/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::str::{self, Utf8Error};
use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json, ParserError};
use rustc_serialize::{Encodable, Encoder};

use minihttp::enums::Status;

pub type Args = Vec<Json>;
pub type Kwargs = BTreeMap<String, Json>;

#[derive(Debug)]
pub struct Message {
    /// Request method;
    method: String,
    /// Client request id;
    request_id: String,
    /// Flag to mark user's session as active;
    active: Option<bool>,

    meta: Kwargs,
    args: Args,
    kwargs: Kwargs,
}

impl Message {

    /// Decode Json str into Message
    pub fn decode(s: &str) -> Result<Message, MessageError>
    {
        let invalid_method = |m: &str| {
            m.starts_with("tangle.") | m.find("/").is_some()
        };
        match Json::from_str(s) {
            Ok(Json::Array(mut message)) => {
                if message.len() != 4 {
                    return Err(MessageError::InvalidLength)
                }
                let kwargs = match message.pop() {
                    Some(Json::Object(kwargs)) => kwargs,
                    _ => return Err(MessageError::ObjectExpected),
                };
                let args = match message.pop() {
                    Some(Json::Array(args)) => args,
                    _ => return Err(MessageError::ArrayExpected),
                };
                let mut meta = match message.pop() {
                    Some(Json::Object(meta)) => meta,
                    _ => return Err(MessageError::ObjectExpected),
                };
                let request_id = match meta.remove("request_id".into()) {
                    Some(Json::String(s)) => s,
                    _ => return Err(MessageError::InvalidRequestId),
                };
                let active = match meta.remove("active".into()) {
                    Some(Json::Boolean(v)) => Some(v),
                    _ => None,  // Maybe return error
                };
                let method = match message.pop() {
                    Some(Json::String(method)) => {
                        if invalid_method(&method) {
                            return Err(MessageError::InvalidMethod);
                        }
                        method
                    }
                    _ => return Err(MessageError::InvalidMethod),
                };
                Ok(Message {
                    method: method,
                    request_id: request_id,
                    active: active,
                    meta: meta,
                    args: args,
                    kwargs: kwargs,
                })
            }
            Ok(_) => Err(MessageError::ArrayExpected),
            Err(e) => Err(MessageError::JsonError(e))
        }
    }

    /// Message request method
    pub fn method(&self) -> &str {
        self.method.as_str()
    }

    /// Message request method converted to path,
    /// ie: `"chat.send_message"` becomes `"chat/send_message"`
    pub fn method_as_path(&self) -> String {
        self.method.replace(".", "/")
    }

    /// Encode message payload for backend request.
    pub fn payload(&self) -> String {
        json::encode(&Payload(self)).unwrap()
    }

    /// Encode backend call result for WebSocket response.
    ///
    /// ```javascript
    /// ["result", {"meta": "data"}, "result obj"]
    /// ```
    pub fn encode_result(&self, result: Json) -> String {
        json::encode(&ResultPayload(self, "result", &result)).unwrap()
    }

    /// Encode backend call error for WebSocket response.
    pub fn encode_error(&self, error: MessageError) -> String {
        json::encode(&ErrorPayload(self, error)).unwrap()
    }
}


#[derive(Debug, PartialEq)]
pub enum MessageError {
    /// Invalid message length;
    InvalidLength,
    /// Invalid method ("tangle." or contains ".");
    InvalidMethod,
    /// Array of args expected;
    ArrayExpected,
    /// Meta/Kwargs object expected;
    ObjectExpected,
    /// Request_id is missing or invalid in request_meta object;
    InvalidRequestId,
    // variants above only for Message parsing; so it makes sense
    // to move those to separate enum and make it smaller

    /// Utf8 decoding error;
    Utf8Error(Utf8Error),
    /// JSON Parser Error;
    JsonError(ParserError),
    /// Response Http Error;
    HttpError(Status, Option<Json>)
}


struct Payload<'a>(&'a Message);

fn encode_meta<S: Encoder>(m: &Message, s: &mut S) -> Result<(), S::Error>
{
    let size = m.meta.len();
    let offset = if m.active.is_some() { 2 } else { 1 };
    s.emit_map(size + offset, |s| {
        try!(s.emit_map_elt_key(
            0, |s| s.emit_str("request_id")));
        try!(s.emit_map_elt_val(
            0, |s| s.emit_str(m.request_id.as_str())));
        if let Some(v) = m.active {
            try!(s.emit_map_elt_key(
                1, |s| s.emit_str("active")));
            try!(s.emit_map_elt_val(
                1, |s| s.emit_bool(v)));
        }
        for (i, (ref k, ref v)) in m.meta.iter().enumerate() {
            try!(s.emit_map_elt_key(
                offset + i, |s| s.emit_str(k.as_str())));
            try!(s.emit_map_elt_val(
                offset + i, |s| v.encode(s)));
        }
        Ok(())
    })
}

impl<'a> Encodable for Payload<'a> {

    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error>
    {
        s.emit_seq(3, |s| {
            try!(s.emit_seq_elt(0, |s| encode_meta(self.0, s)));
            try!(s.emit_seq_elt(1, |s| {
                // emit args
                s.emit_seq(self.0.args.len(), |s| {
                    let args = self.0.args.iter().enumerate();
                    for (idx, ref val) in args {
                        try!(s.emit_seq_elt(idx, |s| val.encode(s)));
                    }
                    Ok(())
                })
            }));
            try!(s.emit_seq_elt(2, |s| {
                // emit kwargs
                s.emit_map(self.0.kwargs.len(), |s| {
                    let items = self.0.kwargs.iter().enumerate();
                    for (idx, (ref k, ref v)) in items {
                        try!(s.emit_map_elt_key(
                            idx, |s| s.emit_str(k.as_str())));
                        try!(s.emit_map_elt_val(
                            idx, |s| v.encode(s)));
                    }
                    Ok(())
                })
            }));
            Ok(())
        })
    }
}

struct ResultPayload<'a>(&'a Message, &'a str, &'a Json);

impl<'a> Encodable for ResultPayload<'a> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(3, |s| {
            try!(s.emit_seq_elt(0, |s| s.emit_str(self.1)));
            try!(s.emit_seq_elt(1, |s| encode_meta(self.0, s)));
            try!(s.emit_seq_elt(2, |s| self.2.encode(s)));
            Ok(())
        })
    }
}

struct ErrorPayload<'a>(&'a Message, MessageError);

impl<'a> Encodable for ErrorPayload<'a> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        use self::MessageError::*;
        s.emit_seq(3, |s| {
            try!(s.emit_seq_elt(0, |s| s.emit_str("error")));
            try!(s.emit_seq_elt(1, |s| {
                let mut size = 1 + self.0.meta.len();
                size += if self.0.active.is_some() { 2 } else { 1 };
                if let HttpError(_, _) = self.1 {
                    size += 1;
                }
                s.emit_map(size, |s| {
                    try!(s.emit_map_elt_key(
                        0, |s| s.emit_str("request_id")));
                    try!(s.emit_map_elt_val(
                        0, |s| s.emit_str(self.0.request_id.as_str())));
                    let mut off = if let Some(v) = self.0.active {
                        try!(s.emit_map_elt_key(1, |s| s.emit_str("active")));
                        try!(s.emit_map_elt_val(1, |s| s.emit_bool(v)));
                        2
                    } else {
                        1
                    };
                    try!(s.emit_map_elt_key(
                        off, |s| s.emit_str("error_kind")));
                    let skip = match self.1 {
                        HttpError(status, _) => {
                            try!(s.emit_map_elt_val(
                                off, |s| s.emit_str("http_error")));
                            off += 1;
                            try!(s.emit_map_elt_key(
                                off, |s| s.emit_str("status")));
                            try!(s.emit_map_elt_val(
                                off, |s| s.emit_u16(status.code())));

                            ["error_kind", "status"]
                        }
                        _ => {
                            try!(s.emit_map_elt_val(
                                off, |s| s.emit_str("invalid_content_type")));

                            ["error_kind", ""]
                        }
                    };
                    let meta_items = self.0.meta.iter()
                        .filter(|&(k, _)| !skip.iter().any(|x| x == k))
                        .enumerate();
                    for (idx, (ref k, ref v)) in meta_items {
                        try!(s.emit_map_elt_key(
                            off + idx, |s| s.emit_str(k.as_str())));
                        try!(s.emit_map_elt_val(
                            off + idx, |s| v.encode(s)));
                    }
                    Ok(())
                })
            }));
            try!(s.emit_seq_elt(1, |s| {
                match self.1 {
                    // yet, only in case of http error we must
                    //  filter out from message.meta 'http_error' key
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
                    ref other => {
                        s.emit_str(format!("{:?}", other).as_str())
                    }
                }
            }));
            Ok(())
        })
    }
}

#[cfg(test)]
mod test {
    use rustc_serialize::json::{Json, ParserError, ErrorCode};
    use minihttp::enums::Status;
    use super::*;

    fn default() -> Message {
        Message::decode(
            "[\"hello\", {\"request_id\": \"req123\"}, [], {}]")
            .unwrap()
    }

    #[test]
    fn decode_message() {
        let msg = Message::decode("[]");
        assert_eq!(msg.err().unwrap(), MessageError::InvalidLength);

        let msg = Message::decode(
            "[\"hello\", {\"request_id\": \"123\"}, [], {}]");
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg.method, "hello".to_string());
        assert_eq!(msg.request_id, "123".to_string());
        assert_eq!(msg.active, None);
        assert_eq!(msg.args.len(), 0);
        assert_eq!(msg.kwargs.len(), 0);
    }

    #[test]
    fn decode_message_errors() {
        let err = Message::decode(
            "[\"bad/method\", {\"request_id\": \"123\"}, [], {}]")
            .err().unwrap();
        assert_eq!(err, MessageError::InvalidMethod);

        let err = Message::decode(
            "[\"good.method\", {\"request_id\": \"123\"}, []]")
            .err().unwrap();
        assert_eq!(err, MessageError::InvalidLength);
    }

    #[test]
    fn method_func() {
        let message = default();

        assert_eq!(message.method(), "hello");
    }

    #[test]
    fn payload() {
        let mut message = default();
        message.meta.insert("debug".into(), Json::Boolean(true));

        let result = message.payload();
        assert_eq!(result, "[{\"request_id\":\"req123\",\"debug\":true},\
            [],{}]".to_string());

        message.args.push(Json::String("msg".to_string()));

        let result = message.payload();
        assert_eq!(result, "[{\"request_id\":\"req123\",\"debug\":true},\
            [\"msg\"],{}]".to_string());

        message.kwargs.insert("name".into(), Json::String("test".to_string()));
        let result = message.payload();
        assert_eq!(result, "[{\"request_id\":\"req123\",\"debug\":true},\
            [\"msg\"],{\"name\":\"test\"}]".to_string());
    }

    #[test]
    fn encode_result() {
        let result = Json::from_str("{\"result\": null}").unwrap();
        let message = default();

        let result = message.encode_result(result);
        assert_eq!(result, "[\"result\",{\"request_id\":\"req123\"},\
            {\"result\":null}]");
    }

    #[test]
    fn encode_error() {
        let mut message = default();
        message.meta.insert("debug".into(), Json::Boolean(true));
        message.meta.insert("error_kind".into(), Json::Boolean(true));
        message.meta.insert("status".into(), Json::Boolean(false));

        let err = MessageError::HttpError(Status::BadRequest, None);
        let result = message.encode_error(err);
        assert_eq!(result, "[\"error\",\
            {\"request_id\":\"req123\",\
            \"error_kind\":\"http_error\",\"status\":400,\
            \"debug\":true},null]");

        let json = Json::from_str("{\"message\":\"data required\"}").unwrap();
        let err = MessageError::HttpError(Status::BadRequest, Some(json));
        let result = message.encode_error(err);
        assert_eq!(result, "[\"error\",{\"request_id\":\"req123\",\
            \"error_kind\":\"http_error\",\"status\":400,\"debug\":true},\
            {\"message\":\"data required\"}]");

        let err = MessageError::JsonError(
            ParserError::SyntaxError(ErrorCode::InvalidSyntax, 0, 0));
        let result = message.encode_error(err);
        assert_eq!(result, "[\"error\",{\"request_id\":\"req123\",\
            \"error_kind\":\"invalid_content_type\",\
            \"debug\":true,\"status\":false},\
            \"SyntaxError(\\\"invalid syntax\\\", 0, 0)\"]");
    }
}
