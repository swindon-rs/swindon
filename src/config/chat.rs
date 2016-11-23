use std::collections::BTreeMap;

use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};
use quire::validate::{Structure, Scalar, Mapping};

use super::http;
use intern::{HandlerName, SessionPoolName};


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Chat {
    pub session_pool: SessionPoolName,
    pub http_route: HandlerName,
    pub message_handlers: BTreeMap<Pattern, http::Destination>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pattern {
    Default,
    Glob(String),
    Exact(String),
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("session_pool", Scalar::new())
    .member("http_route", http::destination_validator())
    .member("message_handlers",
        Mapping::new(Scalar::new(), http::destination_validator()))
}

impl Chat {

    pub fn find_destination(&self, method: &str)
        -> &http::Destination
    {
        let default = self.message_handlers.get(&Pattern::Default).unwrap();
        self.message_handlers.iter().rev()
        .find(|&(k, _)| k.matches(method))
        .map(|(_, v)| v)
        .unwrap_or(default)
    }
}

impl Encodable for Pattern {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match *self {
            Pattern::Default => s.emit_str("*")?,
            Pattern::Glob(ref v) => s.emit_str(format!("{}*", v).as_str())?,
            Pattern::Exact(ref v) => s.emit_str(v.as_str())?,
        }
        Ok(())
    }
}

impl Decodable for Pattern {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let s = d.read_str()?;
        if s.as_str() == "*" {
            Ok(Pattern::Default)
        } else if s.as_str().ends_with(".*") {
            let (p, _) = s.split_at(s.len()-1);
            Ok(Pattern::Glob(p.to_string()))
        } else {
            Ok(Pattern::Exact(s))
        }
    }
}

impl Pattern {
    pub fn matches(&self, other: &str) -> bool {
        match self {
            // Default pattern does not match anything, as its a special case
            &Pattern::Default => false,
            &Pattern::Glob(ref s) => {
                let s = s.as_str();
                other.len() > s.len() &&  other.starts_with(s)
            }
            &Pattern::Exact(ref s) => {
                s.as_str() == other
            }
        }
    }
}

#[cfg(test)]
mod test {
    use rustc_serialize::json;
    use super::Pattern;

    #[test]
    fn decode_pattern() {
        let p: Pattern = json::decode(r#""*""#).unwrap();
        assert_eq!(p, Pattern::Default);

        let p: Pattern = json::decode(r#""hello.world""#).unwrap();
        assert_eq!(p, Pattern::Exact("hello.world".to_string()));

        let p: Pattern = json::decode(r#""hello.world*""#).unwrap();
        assert_eq!(p, Pattern::Exact("hello.world*".to_string()));

        let p: Pattern = json::decode(r#""hello.*""#).unwrap();
        assert_eq!(p, Pattern::Glob("hello.".to_string()));
    }
}
