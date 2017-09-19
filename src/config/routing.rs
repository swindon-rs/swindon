use std::str::FromStr;
use regex::Regex;

use serde::de::{Deserializer, Deserialize};
use quire::validate::{Mapping, Scalar};

use config::visitors::FromStrVisitor;
use intern::{HandlerName, Authorizer};
use routing::RoutingTable;

lazy_static! {
    static ref ROUTING_RE: Regex = Regex::new(
        r"^(?:@([\w-]+)|->([\w-]+)|([\w-]+)=([\w-]*)|([\w-]+))\s*"
        //     #1 auth    #2 logs  #3 named+#4       #5 destination
    ).unwrap();
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Route {
    pub destination: HandlerName,
    pub authorizer: Option<Authorizer>,
}

pub type Routing = RoutingTable<Route>;

pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}

impl<'a> Deserialize<'a> for Route {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(FromStrVisitor::new("route [@authorizer]"))
    }
}

impl FromStr for Route {
    type Err = String;
    fn from_str(val: &str) -> Result<Route, String> {
        let mut val = val.trim();
        let mut destination = None;
        let mut authorizer = None;
        while val.len() > 0 {
            if let Some(m) = ROUTING_RE.captures(val) {
                if let Some(dest) = m.get(5) {
                    if let Some(old) = destination {
                        return Err(format!("Two destinations {:?} and {:?}",
                            old, dest.as_str()));
                    } else {
                        destination = Some(dest.as_str().parse().unwrap());
                    }
                } else if let Some(auth) = m.get(1) {
                    println!("GOT {:?} / {:?}",
                        m.get(1).unwrap().as_str(), m.get(0).unwrap().as_str());
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
        if let Some(dest) = destination {
            return Ok(Route {
                destination: dest,
                authorizer: authorizer,
            })
        } else {
            return Err(String::from("destination is required"));
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;
    use string_intern::Symbol;
    use super::Route;

    #[test]
    fn parse_dest() {
        assert_eq!(Route::from_str("destination").unwrap(), Route {
            destination: Symbol::from("destination"),
            authorizer: None,
        });
    }

    #[test]
    fn parse_auth() {
        assert_eq!(Route::from_str("destination@auth").unwrap(), Route {
            destination: Symbol::from("destination"),
            authorizer: Some(Symbol::from("auth")),
        });
        assert_eq!(Route::from_str("destination   @auth").unwrap(), Route {
            destination: Symbol::from("destination"),
            authorizer: Some(Symbol::from("auth")),
        });
        assert_eq!(Route::from_str("destination @auth").unwrap(), Route {
            destination: Symbol::from("destination"),
            authorizer: Some(Symbol::from("auth")),
        });
    }
}
