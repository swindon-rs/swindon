/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::str;
use std::ascii::AsciiExt;
use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json};
use rustc_serialize::{Encodable, Encoder};

pub type Meta = BTreeMap<String, Json>;
pub type Args = Vec<Json>;
pub type Kwargs = BTreeMap<String, Json>;


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

/// Decode Websocket json message into Meta & Message structs.
pub fn decode_message(s: &str)
    -> Result<(String, Meta, Args, Kwargs), ValidationError>
{
    use self::ValidationError::*;
    let valid_method = |m: &str| {
        !m.starts_with("tangle.") &&
        m.chars().all(|c| c.is_ascii() &&
            (c.is_alphanumeric() || c == '-' || c == '_' || c == '.' ))
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
                Some(&Json::String(ref s)) if s.len() > 0 => {}
                Some(&Json::I64(_)) |
                    Some(&Json::U64(_)) |
                    Some(&Json::F64(_)) => {},
                _ => return Err(InvalidRequestId),
            };
            let method = match message.pop() {
                Some(Json::String(method)) => {
                    if !valid_method(&method) {
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


#[cfg(test)]
mod test {
    use rustc_serialize::json::Json;

    use chat::message::{self, Meta, Args, Kwargs};
    use super::ValidationError as V;

    #[test]
    fn decode_message_errors() {
        macro_rules! err {
            ($a:expr, $b:expr) => {
                assert_eq!(message::decode_message($a).err().unwrap(), $b);
            }
        }
        err!("", V::ArrayExpected);
        err!("[invalid json", V::ArrayExpected);
        err!("{}", V::ArrayExpected);
        err!("[]", V::InvalidLength);
        err!("[1, 2, 3, 4, 5]", V::InvalidLength);
        err!("[1, 2, 3, 4]", V::ObjectExpected);
        err!("[null, null, null, 4]", V::ObjectExpected);
        err!("[1, 2, 3, {}]", V::ArrayExpected);
        err!("[1, 2, [], {}]", V::ObjectExpected);
        err!("[1, {}, [], {}]", V::InvalidRequestId);
        err!("[1, {\"request_id\": null}, [], {}]", V::InvalidRequestId);
        err!("[1, {\"request_id\": []}, [], {}]", V::InvalidRequestId);
        err!("[1, {\"request_id\": {}}, [], {}]", V::InvalidRequestId);
        err!("[1, {\"request_id\": \"\"}, [], {}]", V::InvalidRequestId);
        err!("[1, {\"request_id\": 123}, [], {}]", V::InvalidMethod);
        err!("[null, {\"request_id\": 123}, [], {}]", V::InvalidMethod);
        err!("[[], {\"request_id\": 123}, [], {}]", V::InvalidMethod);
        err!("[{}, {\"request_id\": 123}, [], {}]", V::InvalidMethod);
        err!("[\"bad/method\", {\"request_id\": 123}, [], {}]",
            V::InvalidMethod);
        err!("[\"very bad method\", {\"request_id\": 123}, [], {}]",
            V::InvalidMethod);
        err!("[\"tangle.auth\", {\"request_id\": 123}, [], {}]",
            V::InvalidMethod);
        err!("[\"   bad.method   \", {\"request_id\": 123}, [], {}]",
            V::InvalidMethod);
    }

    #[test]
    fn decode_message() {
        let res = message::decode_message(r#"
            ["some.method", {"request_id": "123"}, ["Hello"], {"world!": "!"}]
            "#).unwrap();
        let (method, meta, args, kwargs) = res;
        assert_eq!(method, "some.method".to_string());
        match meta.get("request_id".into()).unwrap() {
            &Json::String(ref s) => assert_eq!(s, &"123".to_string()),
            _ => unreachable!(),
        }
        assert_eq!(args.len(), 1);
        match kwargs.get("world!".into()).unwrap() {
            &Json::String(ref s) => assert_eq!(s, &"!".to_string()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn encode_auth() {
        let res = message::encode_auth(&"conn:1".to_string(), &Kwargs::new());
        assert_eq!(res, r#"[{"connection_id":"conn:1"},[],{}]"#);

        let mut kw = Kwargs::new();
        kw.insert("http_cookie".into(), Json::String("auth=ok".into()));

        let res = message::encode_auth(&"conn:2".to_string(), &kw);
        assert_eq!(res, concat!(
            r#"[{"connection_id":"conn:2"},"#,
            r#"[],{"http_cookie":"auth=ok"}]"#));
    }

    #[test]
    fn encode_call() {
        let mut meta = Meta::new();
        let mut args = Args::new();
        let mut kw = Kwargs::new();

        let res = message::encode_call(&meta, &args, &kw);
        assert_eq!(res, "[{},[],{}]");

        meta.insert("request_id".into(), Json::String("123".into()));
        args.push(Json::String("Hello".into()));
        args.push(Json::String("World!".into()));
        kw.insert("room".into(), Json::U64(123));

        let res = message::encode_call(&meta, &args, &kw);
        assert_eq!(res, concat!(
            r#"[{"request_id":"123"},"#,
            r#"["Hello","World!"],"#,
            r#"{"room":123}]"#));
    }

    #[test]
    fn get_active() {
        let mut meta = Meta::new();

        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), Json::String("".into()));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), Json::Boolean(true));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), Json::I64(123i64));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), Json::F64(123f64));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), Json::U64(123));
        assert_eq!(message::get_active(&meta).unwrap(), 123 as u64);
    }
}
