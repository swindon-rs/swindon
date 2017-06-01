/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::str;
use std::ascii::AsciiExt;

use serde_json::{Value as Json, Map, from_str as json_decode};
use serde::ser::{Serialize, Serializer, SerializeTuple, SerializeMap};

pub type Meta = Map<String, Json>;
pub type Args = Vec<Json>;
pub type Kwargs = Map<String, Json>;


quick_error! {
    #[derive(Debug, PartialEq)]
    pub enum ValidationError {
        /// Invalid message length;
        InvalidLength {}
        /// Invalid method ("tangle." or contains ".");
        InvalidMethod {}
        /// request_id is missing or invalid in request_meta object;
        InvalidRequestId {}
        /// user_id is missing or invalid in request_meta object;
        InvalidUserId {}
        /// Array of args expected;
        ArrayExpected {}
        /// Meta/Kwargs object expected;
        ObjectExpected {}
    }
}

/// Decode Websocket json message into Meta & Message structs.
pub fn decode_message(s: &str)
    -> Result<(String, Meta, Args, Kwargs), ValidationError>
{
    #[derive(Deserialize)]
    struct Request(String, Meta, Args, Kwargs);

    let res = json_decode(s).map_err(|_| ValidationError::ArrayExpected)?;

    let Request(method, meta, args, kwargs) = res;
    Ok((method, meta, args, kwargs))
    // use self::ValidationError::*;
    // let valid_method = |m: &str| {
    //     !m.starts_with("tangle.") &&
    //     m.chars().all(|c| c.is_ascii() &&
    //         (c.is_alphanumeric() || c == '-' || c == '_' || c == '.' ))
    // };
    // match Json::from_str(s) {
    //     Ok(Json::Array(mut message)) => {
    //         use rustc_serialize::json::Json::*;

    //         if message.len() != 4 {
    //             return Err(InvalidLength)
    //         }
    //         let kwargs = match message.pop() {
    //             Some(Object(kwargs)) => kwargs,
    //             _ => return Err(ObjectExpected),
    //         };
    //         let args = match message.pop() {
    //             Some(Array(args)) => args,
    //             _ => return Err(ArrayExpected),
    //         };
    //         let meta = match message.pop() {
    //             Some(Object(meta)) => meta,
    //             _ => return Err(ObjectExpected),
    //         };
    //         match meta.get("request_id".into()) {
    //             Some(&Json::String(ref s)) if s.len() > 0 => {}
    //             Some(&Json::I64(_)) |
    //                 Some(&Json::U64(_)) |
    //                 Some(&Json::F64(_)) => {},
    //             _ => return Err(InvalidRequestId),
    //         };
    //         let method = match message.pop() {
    //             Some(Json::String(method)) => {
    //                 if !valid_method(&method) {
    //                     return Err(InvalidMethod);
    //                 }
    //                 method
    //             }
    //             _ => return Err(InvalidMethod),
    //         };
    //         Ok((method, meta, args, kwargs))
    //     }
    //     _ => Err(ArrayExpected),
    // }
}


/// Returns true if Meta object contains 'active' key and
/// it either set to true or uint timeout (in seconds).
pub fn get_active(meta: &Meta) -> Option<u64>
{
    meta.get(&"active".to_string()).and_then(|v| v.as_u64())
}


#[derive(Serialize)]
pub struct AuthData {
    pub http_cookie: Option<String>,
    pub http_authorization: Option<String>,
    pub url_querystring: String,
}

// Private tools

pub struct Auth<'a>(pub &'a String, pub &'a AuthData);

impl<'a> Serialize for Auth<'a> {
    fn serialize<S: Serializer>(&self, serializer: S)
        -> Result<S::Ok, S::Error>
    {
        #[derive(Serialize)]
        struct Meta<'a> {
            connection_id: &'a str,
        }
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&Meta { connection_id: self.0.as_str() })?;
        tup.serialize_element(&json!([]))?;
        tup.serialize_element(&self.1)?;
        tup.end()
    }
}

pub struct Call<'a>(pub &'a Meta, pub &'a String, pub &'a Args, pub &'a Kwargs);

// impl<'a> Encodable for Call<'a> {
//     fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
//         s.emit_seq(3, |s| {
//             s.emit_seq_elt(0, |s| {
//                 let n = match self.0.get("connection_id") {
//                     Some(_) => self.0.len(),
//                     None => self.0.len() + 1,
//                 };
//                 s.emit_map(n, |s| {
//                     s.emit_map_elt_key(0, |s| s.emit_str("connection_id"))?;
//                     s.emit_map_elt_val(0, |s| self.1.encode(s))?;
//                     let m = self.0.iter()
//                         .filter(|&(&ref k, _)| k != "connection_id")
//                         .enumerate();
//                     for (i, (ref k, ref v)) in m {
//                         s.emit_map_elt_key(i+1, |s| s.emit_str(k))?;
//                         s.emit_map_elt_val(i+1, |s| v.encode(s))?;
//                     }
//                     Ok(())
//                 })
//             })?;
//             s.emit_seq_elt(1, |s| self.2.encode(s))?;
//             s.emit_seq_elt(2, |s| self.3.encode(s))?;
//             Ok(())
//         })?;
//         Ok(())
//     }
// }
impl<'a> Serialize for Call<'a> {
    fn serialize<S: Serializer>(&self, serializer: S)
        -> Result<S::Ok, S::Error>
    {
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&self.0)?;
        tup.serialize_element(&self.2)?;
        tup.serialize_element(&self.3)?;
        tup.end()
    }
}


#[cfg(test)]
mod test {
    // use serde_json::Value as Json;
    // use serde_json::to_string as json_encode;

    // use chat::message::{self, Call, Meta, Args, Kwargs, Auth, AuthData};
    // use super::ValidationError as V;

    // #[test]
    // fn decode_message_errors() {
    //     macro_rules! err {
    //         ($a:expr, $b:expr) => {
    //             assert_eq!(message::decode_message($a).err().unwrap(), $b);
    //         }
    //     }
    //     err!("", V::ArrayExpected);
    //     err!("[invalid json", V::ArrayExpected);
    //     err!("{}", V::ArrayExpected);
    //     err!("[]", V::InvalidLength);
    //     err!("[1, 2, 3, 4, 5]", V::InvalidLength);
    //     err!("[1, 2, 3, 4]", V::ObjectExpected);
    //     err!("[null, null, null, 4]", V::ObjectExpected);
    //     err!("[1, 2, 3, {}]", V::ArrayExpected);
    //     err!("[1, 2, [], {}]", V::ObjectExpected);
    //     err!("[1, {}, [], {}]", V::InvalidRequestId);
    //     err!("[1, {\"request_id\": null}, [], {}]", V::InvalidRequestId);
    //     err!("[1, {\"request_id\": []}, [], {}]", V::InvalidRequestId);
    //     err!("[1, {\"request_id\": {}}, [], {}]", V::InvalidRequestId);
    //     err!("[1, {\"request_id\": \"\"}, [], {}]", V::InvalidRequestId);
    //     err!("[1, {\"request_id\": 123}, [], {}]", V::InvalidMethod);
    //     err!("[null, {\"request_id\": 123}, [], {}]", V::InvalidMethod);
    //     err!("[[], {\"request_id\": 123}, [], {}]", V::InvalidMethod);
    //     err!("[{}, {\"request_id\": 123}, [], {}]", V::InvalidMethod);
    //     err!("[\"bad/method\", {\"request_id\": 123}, [], {}]",
    //         V::InvalidMethod);
    //     err!("[\"very bad method\", {\"request_id\": 123}, [], {}]",
    //         V::InvalidMethod);
    //     err!("[\"tangle.auth\", {\"request_id\": 123}, [], {}]",
    //         V::InvalidMethod);
    //     err!("[\"   bad.method   \", {\"request_id\": 123}, [], {}]",
    //         V::InvalidMethod);
    // }

    // #[test]
    // fn decode_message() {
    //     let res = message::decode_message(r#"
    //         ["some.method", {"request_id": "123"}, ["Hello"], {"world!": "!"}]
    //         "#).unwrap();
    //     let (method, meta, args, kwargs) = res;
    //     assert_eq!(method, "some.method".to_string());
    //     match meta.get("request_id".into()).unwrap() {
    //         &Json::String(ref s) => assert_eq!(s, &"123".to_string()),
    //         _ => unreachable!(),
    //     }
    //     assert_eq!(args.len(), 1);
    //     match kwargs.get("world!".into()).unwrap() {
    //         &Json::String(ref s) => assert_eq!(s, &"!".to_string()),
    //         _ => unreachable!(),
    //     }
    // }

    // #[test]
    // fn encode_auth() {
    //     let res = json_encode(&Auth(&"conn:1".to_string(), &AuthData {
    //         http_cookie: None, http_authorization: None,
    //         url_querystring: "".to_string(),
    //     })).unwrap();
    //     assert_eq!(res, concat!(
    //         r#"[{"connection_id":"conn:1"},[],{"#,
    //         r#""http_cookie":null,"http_authorization":null,"#,
    //         r#""url_querystring":""}]"#));

    //     let kw = AuthData {
    //         http_cookie: Some("auth=ok".to_string()),
    //         http_authorization: None,
    //         url_querystring: "".to_string(),
    //     };

    //     let res = json_encode(&Auth(&"conn:2".to_string(), &kw)).unwrap();
    //     assert_eq!(res, concat!(
    //         r#"[{"connection_id":"conn:2"},"#,
    //         r#"[],{"http_cookie":"auth=ok","#,
    //         r#""http_authorization":null,"url_querystring":""}]"#));
    // }

    // #[test]
    // fn encode_call() {
    //     let mut meta = Meta::new();
    //     let mut args = Args::new();
    //     let mut kw = Kwargs::new();
    //     let cid = "123".to_string();

    //     let res = json_encode(&Call(&meta, &cid, &args, &kw)).unwrap();
    //     assert_eq!(res, "[{\"connection_id\":\"123\"},[],{}]");

    //     meta.insert("request_id".into(), Json::String("123".into()));
    //     args.push(Json::String("Hello".into()));
    //     args.push(Json::String("World!".into()));
    //     kw.insert("room".into(), Json::U64(123));

    //     let res = json_encode(&Call(&meta, &cid, &args, &kw)).unwrap();
    //     assert_eq!(res, concat!(
    //         r#"[{"connection_id":"123","request_id":"123"},"#,
    //         r#"["Hello","World!"],"#,
    //         r#"{"room":123}]"#));

    //     meta.insert("connection_id".into(), Json::String("321".into()));
    //     let res = json_encode(&Call(&meta, &cid, &args, &kw)).unwrap();
    //     assert_eq!(res, concat!(
    //         r#"[{"connection_id":"123","request_id":"123"},"#,
    //         r#"["Hello","World!"],"#,
    //         r#"{"room":123}]"#));
    // }

    // #[test]
    // fn get_active() {
    //     let mut meta = Meta::new();

    //     assert!(message::get_active(&meta).is_none());

    //     meta.insert("active".into(), Json::String("".into()));
    //     assert!(message::get_active(&meta).is_none());

    //     meta.insert("active".into(), Json::Boolean(true));
    //     assert!(message::get_active(&meta).is_none());

    //     meta.insert("active".into(), Json::I64(123i64));
    //     assert!(message::get_active(&meta).is_none());

    //     meta.insert("active".into(), Json::F64(123f64));
    //     assert!(message::get_active(&meta).is_none());

    //     meta.insert("active".into(), Json::U64(123));
    //     assert_eq!(message::get_active(&meta).unwrap(), 123u64);

    //     if let Json::Object(meta) = Json::from_str("{\"active\": 123}")
    //         .unwrap()
    //     {
    //         assert!(message::get_active(&meta).is_some());
    //     }
    // }
}
