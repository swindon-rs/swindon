use std::str::FromStr;
use regex::Regex;

use serde::de::{Deserializer, Deserialize};
use quire::validate::{Mapping, Scalar};

use config::visitors::FromStrVisitor;
use intern::{HandlerName, Authorizer};

lazy_static! {
    static ref ROUTING_RE: Regex = Regex::new(
        r"^(?:@([\w-]+)|->([\w-]+)|([\w-]+)=([\w-]*)|([\w-]+))\s*"
        //     #1 auth    #2 logs  #3 named+#4       #5 destination
    ).unwrap();
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RouteDef {
    pub handler: HandlerName,
    pub authorizer: Option<Authorizer>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Host(pub bool, pub String);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct HostPath(pub Host, pub Option<String>);

impl Host {
    pub fn matches_www(&self) -> bool {
        self.0 || self.1.starts_with("www.")
    }
}

impl<'a> Deserialize<'a> for HostPath {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(FromStrVisitor::new(
            "hostname or hostname/path"))
    }
}

impl FromStr for Host {
    type Err = String;

    fn from_str(val: &str) -> Result<Host, String> {
        if val == "*" {
            Ok(Host(true, String::from("")))
        } else if val.starts_with("*.") {
            Ok(Host(true, val[2..].to_string()))
        } else {
            Ok(Host(false, val.to_string()))
        }
    }
}


impl FromStr for HostPath {
    type Err = String;
    fn from_str(val: &str) -> Result<HostPath, String> {
        let (host, path) = if let Some(i) = val.find('/') {
            if &val[i..] == "/" {
                (&val[..i], None)
            } else {
                (&val[..i], Some(val[i..].to_string()))
            }
        } else {
            (val, None)
        };
        Ok(HostPath(host.parse().unwrap(), path))
    }
}


pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}

impl<'a> Deserialize<'a> for RouteDef {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(FromStrVisitor::new("route [@authorizer]"))
    }
}

impl FromStr for RouteDef {
    type Err = String;
    fn from_str(val: &str) -> Result<RouteDef, String> {
        let mut val = val.trim();
        let mut handler = None;
        let mut authorizer = None;
        while val.len() > 0 {
            if let Some(m) = ROUTING_RE.captures(val) {
                if let Some(dest) = m.get(5) {
                    if let Some(old) = handler {
                        return Err(format!("Two handlers {:?} and {:?}",
                            old, dest.as_str()));
                    } else {
                        handler = Some(dest.as_str().parse().unwrap());
                    }
                } else if let Some(auth) = m.get(1) {
                    if let Some(old) = authorizer {
                        return Err(format!("Two authorizers {:?} and {:?}",
                            old, auth.as_str()));
                    } else {
                        authorizer = Some(auth.as_str().parse().unwrap());
                    }
                } else if let Some(_) = m.get(2) {
                    panic!("Logs are not implemented yet");
                } else if let Some(name) = m.get(3) {
                    panic!("Key {:?} is not implemented yet", name);
                }
                val = &val[m.get(0).unwrap().end()..];
            } else {
                return Err(format!("Unexpected token {:?}", val));
            }
        }
        if let Some(dest) = handler {
            return Ok(RouteDef {
                handler: dest,
                authorizer: authorizer,
            })
        } else {
            return Err(String::from("handler is required"));
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;
    use string_intern::Symbol;
    use super::RouteDef;

    #[test]
    fn parse_dest() {
        assert_eq!(RouteDef::from_str("handler").unwrap(), RouteDef {
            handler: Symbol::from("handler"),
            authorizer: None,
        });
    }

    #[test]
    fn parse_auth() {
        assert_eq!(RouteDef::from_str("handler@auth").unwrap(), RouteDef {
            handler: Symbol::from("handler"),
            authorizer: Some(Symbol::from("auth")),
        });
        assert_eq!(RouteDef::from_str("handler   @auth").unwrap(),
            RouteDef {
                handler: Symbol::from("handler"),
                authorizer: Some(Symbol::from("auth")),
            });
        assert_eq!(RouteDef::from_str("handler @auth").unwrap(), RouteDef {
            handler: Symbol::from("handler"),
            authorizer: Some(Symbol::from("auth")),
        });
    }
}

#[cfg(test)]
mod parse_test {
    use super::{HostPath, Host};

    fn parse_host_path(s: String) -> (Host, Option<String>) {
        let HostPath(host, path) = s.parse().unwrap();
        return (host, path);
    }

    #[test]
    fn simple() {
        let s = "example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host(false, "example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn base_host() {
        let s = "*.example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host(true, "example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn invalid_base_host() {
        let s = "*example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host(false, "*example.com".into()));
        assert!(path.is_none());

        let s = ".example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host(false, ".example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn invalid_host() {
        // FiXME: only dot is invalid
        let s = "*.".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host(true, "".into()));
        assert!(path.is_none());

        let s = "*./".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host(true, "".into()));
        assert!(path.is_none());
    }

}
