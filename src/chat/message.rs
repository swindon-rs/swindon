/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::str;
use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json};
use rustc_serialize::{Encodable, Encoder};

use super::error::ValidationError;

pub type Meta = BTreeMap<String, Json>;
pub type Args = Vec<Json>;
pub type Kwargs = BTreeMap<String, Json>;

/// Decode Websocket json message into Meta & Message structs.
pub fn decode_message(s: &str)
    -> Result<(String, Meta, Args, Kwargs), ValidationError>
{
    use super::error::ValidationError::*;
    let invalid_method = |m: &str| {
        m.starts_with("tangle.") | m.find("/").is_some()
    };
    match Json::from_str(s) {
        Ok(Json::Array(mut message)) => {
            use rustc_serialize::json::Json::*;

            if message.len() != 4 {
                return Err(InvalidLength)
            }
            let kwargs = match message.pop() {
                Some(Object(kwargs)) => kwargs,
                _ => return Err(ObjectExpected),
            };
            let args = match message.pop() {
                Some(Array(args)) => args,
                _ => return Err(ArrayExpected),
            };
            let meta = match message.pop() {
                Some(Object(meta)) => meta,
                _ => return Err(ObjectExpected),
            };
            match meta.get("request_id".into()) {
                Some(&Json::String(_)) |
                    Some(&Json::I64(_)) |
                    Some(&Json::U64(_)) |
                    Some(&Json::F64(_)) => {},
                _ => return Err(InvalidRequestId),
            };
            let method = match message.pop() {
                Some(Json::String(method)) => {
                    if invalid_method(&method) {
                        return Err(InvalidMethod);
                    }
                    method
                }
                _ => return Err(InvalidMethod),
            };
            Ok((method, meta, args, kwargs))
        }
        _ => Err(ArrayExpected),
    }
}


/// Returns true if Meta object contains 'active' flag and is set to true.
pub fn get_active(meta: &Meta) -> Option<u64> {
    let duration = meta.get(&"active".to_string());
    match duration {
        Some(&Json::U64(v)) => Some(v),
        _ => None,
    }
}

/// Encode to JSON Auth message.
pub fn encode_auth(connection_id: &String, data: &Kwargs) -> String {
    json::encode(&Auth(connection_id, data)).unwrap()
}

/// Encode to JSON Websocket call:
/// `[<meta_obj>, <args_list>, <kwargs_obj>]`
pub fn encode_call(meta: &Meta, args: &Args, kwargs: &Kwargs) -> String {
    json::encode(&Call(meta, args, kwargs)).unwrap()
}

// Private tools

struct Auth<'a>(&'a String, &'a Kwargs);

impl<'a> Encodable for Auth<'a> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(3, |s| {
            s.emit_seq_elt(0, |s| {
                s.emit_map(1, |s| {
                    s.emit_map_elt_key(0, |s| s.emit_str("connection_id"))?;
                    s.emit_map_elt_val(0, |s| self.0.encode(s))?;
                    Ok(())
                })?;
                Ok(())
            })?;
            s.emit_seq_elt(1, |s| s.emit_seq(0, |_| Ok(())))?;
            s.emit_seq_elt(2, |s| self.1.encode(s))?;
            Ok(())
        })?;
        Ok(())
    }
}

struct Call<'a>(&'a Meta, &'a Args, &'a Kwargs);

impl<'a> Encodable for Call<'a> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(3, |s| {
            s.emit_seq_elt(0, |s| self.0.encode(s))?;
            s.emit_seq_elt(1, |s| self.1.encode(s))?;
            s.emit_seq_elt(2, |s| self.2.encode(s))?;
            Ok(())
        })?;
        Ok(())
    }
}
