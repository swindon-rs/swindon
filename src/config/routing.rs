use std::str::FromStr;
use std::collections::BTreeMap;

use intern::HandlerName;
use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Mapping, Scalar};


#[derive(Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct Route {
    pub is_base: bool,
    pub host: String,
    pub path: Option<String>,
}

pub type Routing = BTreeMap<Route, HandlerName>;

pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}

impl Decodable for Route {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_str()?
        .parse()
        .map_err(|e: String| d.error(&e))
    }
}

impl FromStr for Route {
    type Err = String;
    fn from_str(val: &str) -> Result<Route, String> {
        let (is_base, val) = if val.starts_with("*.") {
            (true, &val[1..])
        } else {
            (false, val)
        };
        if let Some(path_start) = val.find('/') {
            if &val[path_start..] == "/" {
                Ok(Route {
                    is_base: is_base,
                    host: val[..path_start].to_string(),
                    path: None,
                })
            } else {
                Ok(Route {
                    is_base: is_base,
                    host: val[..path_start].to_string(),
                    path: Some(val[path_start..].to_string()),
                })
            }
        } else {
            Ok(Route {
                is_base: is_base,
                host: val.to_string(),
                path: None,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::Route;

    #[test]
    fn simple() {
        let s = "example.com";
        let route: Route = s.parse().unwrap();
        assert!(!route.is_base);
        assert_eq!(route.host, "example.com");
        assert!(route.path.is_none());
    }

    #[test]
    fn base_host() {
        let s = "*.example.com";
        let route: Route = s.parse().unwrap();
        assert!(route.is_base);
        assert_eq!(route.host, ".example.com");
        assert!(route.path.is_none());
    }

    #[test]
    fn invalid_base_host() {
        let s = "*example.com";
        let route: Route = s.parse().unwrap();
        assert!(!route.is_base);
        assert_eq!(route.host, "*example.com");
        assert!(route.path.is_none());

        let s = ".example.com";
        let route: Route = s.parse().unwrap();
        assert!(!route.is_base);
        assert_eq!(route.host, ".example.com");
        assert!(route.path.is_none());
    }

    #[test]
    fn invalid_host() {
        let s = "*.";
        let route: Route = s.parse().unwrap();
        assert!(route.is_base);
        assert_eq!(route.host, ".");
        assert!(route.path.is_none());

        let s = "*./";
        let route: Route = s.parse().unwrap();
        assert!(route.is_base);
        assert_eq!(route.host, ".");
        assert!(route.path.is_none());
    }
}
