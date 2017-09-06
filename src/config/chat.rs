use std::collections::BTreeMap;
use std::ops::Deref;
use std::str::FromStr;

use serde::de::{Deserialize, Deserializer, Error};
use quire::validate::{Structure, Scalar, Mapping};

use super::http;
use intern::{HandlerName, SessionPoolName};
use config::visitors::FromStrVisitor;


#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct Chat {
    pub allow_empty_subprotocol: bool,
    pub use_tangle_prefix: bool,
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
    pub default: http::Destination,
    pub map: BTreeMap<Pattern, http::Destination>,
}

pub fn old_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("allow_empty_subprotocol", Scalar::new().default(true))
    .member("use_tangle_prefix", Scalar::new().default(true))
    .member("session_pool", Scalar::new())
    .member("session_pool", Scalar::new())
    .member("http_route", http::destination_validator().optional())
    .member("message_handlers",
        Mapping::new(Scalar::new(), http::destination_validator()))
}

pub fn validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("allow_empty_subprotocol", Scalar::new().default(false))
    .member("use_tangle_prefix", Scalar::new().default(false))
    .member("session_pool", Scalar::new())
    .member("session_pool", Scalar::new())
    .member("http_route", http::destination_validator().optional())
    .member("message_handlers",
        Mapping::new(Scalar::new(), http::destination_validator()))
}

/*
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
*/

impl FromStr for Pattern {
    type Err = String;
    fn from_str(s: &str) -> Result<Pattern, String> {
        if s == "*" {
            Ok(Pattern::Default)
        } else if s.ends_with(".*") {
            let (p, _) = s.split_at(s.len()-1);
            Ok(Pattern::Glob(p.to_string()))
        } else {
            Ok(Pattern::Exact(s.to_string()))
        }
    }
}

impl<'a> Deserialize<'a> for Pattern {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(FromStrVisitor::new(
            "exact string, or asterisk, or pattern that ends with `.*`"))
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

impl<'a> Deserialize<'a> for RoutingTable {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        let mut tmp = BTreeMap::<Pattern, http::Destination>::deserialize(d)?;
        let default = tmp.remove(&Pattern::Default)
            .ok_or(D::Error::custom("No default route"))?;
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
    use serde_json::from_str;
    use super::Pattern;

    #[test]
    fn decode_pattern() {
        let p: Pattern = from_str(r#""*""#).unwrap();
        assert_eq!(p, Pattern::Default);

        let p: Pattern = from_str(r#""hello.world""#).unwrap();
        assert_eq!(p, Pattern::Exact("hello.world".to_string()));

        let p: Pattern = from_str(r#""hello.world*""#).unwrap();
        assert_eq!(p, Pattern::Exact("hello.world*".to_string()));

        let p: Pattern = from_str(r#""hello.*""#).unwrap();
        assert_eq!(p, Pattern::Glob("hello.".to_string()));
    }
}
