/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::str;
use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json};
use rustc_serialize::{Encodable, Encoder};

use super::error::MessageError;

pub type Meta = BTreeMap<String, Json>;
pub type Args = Vec<Json>;
pub type Kwargs = BTreeMap<String, Json>;

// BackendMessage
pub enum Message {
    /// Websocket call.
    Call(String, Args, Kwargs),
    /// Auth message
    Auth(Kwargs),
    /// Session inactive message.
    Inactive,
}


impl Message {

    /// Message request method
    pub fn method(&self) -> &str {
        match *self {
            Message::Call(ref m, _, _) => m.as_str(),
            Message::Auth(_) => "tangle.authorize_connection",
            Message::Inactive => "tangle.inactive",
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
}

/// Decode Websocket json message into Meta & Message structs.
pub fn decode_message(s: &str)
    -> Result<(Meta, Message), MessageError>
{
    use super::error::ValidationError::*;
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


/// Returns true if Meta object contains 'active' flag and is set to true.
pub fn is_active(meta: &Meta) -> bool {
    match meta.get(&"active".to_string()) {
        Some(&Json::Boolean(v)) => v,
        _ => false,
    }
}

// Private tools


struct Payload<'a>(Option<&'a Meta>, &'a Message);

impl<'a> Encodable for Payload<'a> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        use self::Message::*;

        s.emit_seq(3, |s| {
            s.emit_seq_elt(0, |s| self.encode_meta(s))?;
            match self.1 {
                &Call(_, ref args, ref kwargs) => {
                    s.emit_seq_elt(1, |s| args.encode(s))?;
                    s.emit_seq_elt(2, |s| kwargs.encode(s))?;
                }
                &Auth(ref kwargs) => {
                    s.emit_seq_elt(1, |s| s.emit_seq(0, |_| Ok(())) )?;
                    s.emit_seq_elt(2, |s| kwargs.encode(s))?;
                }
                &Inactive => {
                    s.emit_seq_elt(1, |s| s.emit_seq(0, |_| Ok(())) )?;
                    s.emit_seq_elt(2, |s| s.emit_map(0, |_| Ok(())) )?;
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
