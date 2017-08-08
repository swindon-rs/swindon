/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::str;
use std::ascii::AsciiExt;
use serde_json::{self, Value as Json, Map, Error as JsonError};
use serde::ser::{Serialize, Serializer, SerializeTuple};

use super::cid::Cid;
use runtime::ServerId;

pub type Meta = Map<String, Json>;
pub type Args = Vec<Json>;
pub type Kwargs = Map<String, Json>;


/// Decode Websocket json message into Meta & Message structs.
pub fn decode_message(s: &str)
    -> Result<(String, Meta, Args, Kwargs), JsonError>
{

    let res = serde_json::from_str::<Request>(s)?;
    let Request(method, meta, args, kwargs) = res;
    Ok((method, meta, args, kwargs))
}


/// Returns true if Meta object contains 'active' key and
/// it either set to true or uint timeout (in seconds).
pub fn get_active(meta: &Meta) -> Option<u64>
{
    meta.get(&"active".to_string()).and_then(|v| v.as_u64())
}

/// Returns true if method is valid.
pub fn valid_method(method: &str) -> bool {
    if method.len() == 0 {
        false
    } else if method.starts_with("tangle.") {
        false
    } else if method.starts_with("swindon.") {
        false
    } else {
        method.chars().all(|c| c.is_ascii() &&
            (c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
    }
}

/// Returns true if request_id is either `u64`
/// or `String` up to 36 characters matching regex [a-z0-9_-]
pub fn valid_request_id(meta: &Meta) -> bool {
     match meta.get("request_id") {
        Some(&Json::String(ref s)) => {
            if s.len() == 0 || s.len() > 36 {
                return false
            }
            s.chars().all(|c| c.is_digit(36) || c == '-' || c == '_')
        }
        Some(&Json::Number(ref n)) => {
            n.is_u64()
        }
        _ => false,
     }
}

#[derive(Serialize)]
pub struct AuthData {
    pub http_cookie: Option<String>,
    pub http_authorization: Option<String>,
    pub url_querystring: String,
}

// Private tools

pub struct Auth<'a>(pub &'a Cid, pub &'a ServerId, pub &'a AuthData);

impl<'a> Serialize for Auth<'a> {
    fn serialize<S: Serializer>(&self, serializer: S)
        -> Result<S::Ok, S::Error>
    {
        let &Auth(cid, sid, auth) = self;
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&json!({
            "connection_id": format!("{}-{}", sid, cid)}))?;
        tup.serialize_element(&json!([]))?;
        tup.serialize_element(auth)?;
        tup.end()
    }
}

pub struct Call<'a>(
    pub &'a Meta, pub &'a Cid, pub &'a ServerId, pub &'a Args, pub &'a Kwargs);

impl<'a> Serialize for Call<'a> {
    fn serialize<S: Serializer>(&self, serializer: S)
        -> Result<S::Ok, S::Error>
    {
        let &Call(meta, cid, sid, args, kwargs) = self;
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&MetaWithExtra {
            meta: meta,
            extra: json!({"connection_id": format!("{}-{}", sid, cid)}),
        })?;
        tup.serialize_element(args)?;
        tup.serialize_element(kwargs)?;
        tup.end()
    }
}

pub struct MetaWithExtra<'a> {
    pub meta: &'a Meta,
    pub extra: Json,
}
impl<'a> Serialize for MetaWithExtra<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        if let Json::Object(ref extra) = self.extra {
            s.collect_map(extra.iter()
                .chain(self.meta.iter()
                    .filter(|&(&ref k,_)| extra.get(k).is_none())))
        } else {
            s.collect_map(self.meta.iter())
        }
    }
}

#[derive(Deserialize)]
pub struct Request(pub String, pub Meta, pub Args, pub Kwargs);

// impl Request {
//     fn validate(&self) -> Result<(), JsonError> {
//         let method = self.0.as_str();
//         if !valid_method(method) {
//             return Err(JsonError::custom("invalid method"))
//         }
//         match self.1.get("request_id") {
//             Some(&Json::Number(_)) => {},
//             Some(&Json::String(ref s)) if s.len() > 0 => {}
//             _ => return Err(JsonError::custom("invalid request_id"))
//         }
//         Ok(())
//     }
// }


#[cfg(test)]
mod test {
    use serde_json::Value as Json;
    use serde_json::to_string as json_encode;

    use request_id;
    use chat::message::{self, Call, Meta, Args, Kwargs, Auth, AuthData};

    #[test]
    fn decode_message_errors() {
        macro_rules! error_starts {
            ($a:expr, $b:expr) => {
                {
                    let rv = message::decode_message($a);
                    assert!(rv.is_err(),
                        format!("unexpectedly valid: {}", $a));
                    let err = format!("{}", rv.err().unwrap());
                    assert!(err.starts_with($b),
                        format!("{}: {} != {}", $a, err, $b));
                }
            };
            ($( $a:expr, $b:expr ),+) => {
                $( error_starts!($a, $b) );*
            };
        }

        error_starts!(
            "",
                "EOF while parsing a value"
        );
        error_starts!(
            "[invalid json",
                "expected value at line 1"
        );
        error_starts!(
            "{}",
                "invalid type: map, expected tuple struct"
        );
        error_starts!(
            "[]",
                "invalid length 0"
        );
        error_starts!(
            "[1, 2, 3, 4, 5]",
                "invalid type: integer `1`, expected a string"
        );
        error_starts!(
            "[1, 2, 3, 4]",
                "invalid type: integer `1`, expected a string"
        );
        error_starts!(
            "[null, null, null, 4]",
                "invalid type: unit value, expected a string"
        );
        error_starts!(
            "[\"1\", 2, 3, 4]",
                "invalid type: integer `2`, expected a map"
        );
        error_starts!(
            "[\"1\", {}, 3, 4]",
                "invalid type: integer `3`, expected a sequence"
        );
        error_starts!(
            "[\"1\", {}, [], 4]",
                "invalid type: integer `4`, expected a map"
        );
    }

    #[test]
    fn decode_message() {
        let res = message::decode_message(r#"
            ["some.method", {"request_id": "123"}, ["Hello"], {"world!": "!"}]
            "#).unwrap();
        let (method, meta, args, kwargs) = res;
        assert_eq!(method, "some.method".to_string());
        match meta.get("request_id").unwrap() {
            &Json::String(ref s) => assert_eq!(s, &"123".to_string()),
            _ => unreachable!(),
        }
        assert_eq!(args.len(), 1);
        match kwargs.get("world!").unwrap() {
            &Json::String(ref s) => assert_eq!(s, &"!".to_string()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn encode_auth() {
        let cid = "1".parse().unwrap();
        let sid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap();

        let res = json_encode(&Auth(&cid, &sid, &AuthData {
            http_cookie: None, http_authorization: None,
            url_querystring: "".to_string(),
        })).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1"},[],{"#,
            r#""http_cookie":null,"http_authorization":null,"#,
            r#""url_querystring":""}]"#));

        let kw = AuthData {
            http_cookie: Some("auth=ok".to_string()),
            http_authorization: None,
            url_querystring: "".to_string(),
        };

        let cid = "2".parse().unwrap();
        let res = json_encode(&Auth(&cid, &sid, &kw)).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-2"},"#,
            r#"[],{"http_cookie":"auth=ok","#,
            r#""http_authorization":null,"url_querystring":""}]"#));
    }

    #[test]
    fn encode_call() {
        let mut meta = Meta::new();
        let mut args = Args::new();
        let mut kw = Kwargs::new();
        let cid = "123".parse().unwrap();
        let sid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap();

        let res = json_encode(&Call(&meta, &cid, &sid, &args, &kw)).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-123"},"#,
            r#"[],{}]"#));

        meta.insert("request_id".into(), json!("123"));
        args.push(json!("Hello"));
        args.push(json!("World!"));
        kw.insert("room".into(), json!(123));

        let res = json_encode(&Call(&meta, &cid, &sid, &args, &kw)).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-123","#,
            r#""request_id":"123"},"#,
            r#"["Hello","World!"],"#,
            r#"{"room":123}]"#));

        meta.insert("connection_id".into(), json!("321"));
        let res = json_encode(&Call(&meta, &cid, &sid, &args, &kw)).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-123","#,
            r#""request_id":"123"},"#,
            r#"["Hello","World!"],"#,
            r#"{"room":123}]"#));
    }

    #[test]
    fn get_active() {
        let mut meta = Meta::new();

        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(""));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(true));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(123i64));
        assert_eq!(message::get_active(&meta).unwrap(), 123u64);

        meta.insert("active".into(), json!(-123));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(123f64));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(123));
        assert_eq!(message::get_active(&meta).unwrap(), 123u64);
    }

    #[test]
    fn valid_method() {
        assert!(message::valid_method("some-method"));
        assert!(message::valid_method("123.456.789"));

        assert!(!message::valid_method(""));
        assert!(!message::valid_method("bad/method"));
        assert!(!message::valid_method("another bad method"));
        assert!(!message::valid_method("tangle.auth"));
        assert!(!message::valid_method("swindon.auth"));
        assert!(!message::valid_method("   tangle.auth"));
        assert!(!message::valid_method("   bad.method   "));
    }

    #[test]
    fn valid_request_id() {
        let mut meta = Meta::new();
        meta.insert("request_id".into(), json!("abc"));
        assert!(message::valid_request_id(&meta));
        meta.insert("request_id".into(),
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(message::valid_request_id(&meta));
        meta.insert("request_id".into(), json!("abc-123_def"));
        assert!(message::valid_request_id(&meta));

        meta.clear();
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(), Json::Null);
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(), json!([]));
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(), json!({}));
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(), Json::String("".into()));
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(), Json::String("i n v a l i d".into()));
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(),
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaA"));
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(), json!(-1));
        assert!(!message::valid_request_id(&meta));
        meta.insert("request_id".into(), json!(1.1));
        assert!(!message::valid_request_id(&meta));
    }
}
