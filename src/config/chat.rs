use std::collections::BTreeMap;
use std::ops::Deref;

use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};
use quire::validate::{Structure, Scalar, Mapping};

use super::http;
use intern::{HandlerName, SessionPoolName};


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct Chat {
    pub session_pool: SessionPoolName,
    pub http_route: Option<HandlerName>,
    pub message_handlers: RoutingTable,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pattern {
    Default,
    Glob(String),
    Exact(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct RoutingTable {
    default: http::Destination,
    map: BTreeMap<Pattern, http::Destination>,
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("session_pool", Scalar::new())
    .member("http_route", http::destination_validator().optional())
    .member("message_handlers",
        Mapping::new(Scalar::new(), http::destination_validator()))
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
            // Default pattern does not match anything,
            //  as its a special case and MUST be used as last resort effort.
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

impl Decodable for RoutingTable {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let mut tmp = BTreeMap::<Pattern, http::Destination>::decode(d)?;
        let default = tmp.remove(&Pattern::Default)
            .ok_or(d.error("No default route"))?;
        Ok(RoutingTable {
            default: default,
            map: tmp,
        })
    }
}

impl Deref for RoutingTable {
    type Target = BTreeMap<Pattern, http::Destination>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl RoutingTable {
    pub fn resolve(&self, method: &str) -> &http::Destination {
        self.iter().rev()
        .find(|&(k, _)| k.matches(method))
        .map(|(_, v)| v)
        .unwrap_or(&self.default)
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
