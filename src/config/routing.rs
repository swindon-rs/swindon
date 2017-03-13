use std::collections::BTreeMap;
use std::ops::Deref;

use intern::HandlerName;
use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Mapping, Scalar};


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Host {
    Suffix(String),
    Exact(String),
}

pub type Path = Option<String>;

#[derive(Debug, PartialEq, Eq)]
pub struct Routing(pub BTreeMap<Host, BTreeMap<Path, HandlerName>>);


pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}

impl Decodable for Routing {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_map(|mut d, n| {
            let mut rv = BTreeMap::new();
            for idx in 0..n {
                let (host, path) = d.read_map_elt_key(idx, |mut d| {
                    d.read_str().map(parse_host_path)
                })?;
                let val = d.read_map_elt_val(idx, HandlerName::decode)?;
                rv.entry(host)
                .or_insert_with(|| BTreeMap::new())
                .insert(path, val);
            }
            Ok(Routing(rv))
        })
    }
}

fn parse_host_path(val: String) -> (Host, Path) {
    let (host, path) = if let Some(i) = val.find('/') {
        if &val[i..] == "/" {
            (&val[..i], None)
        } else {
            (&val[..i], Some(val[i..].to_string()))
        }
    } else {
        (val.as_str(), None)
    };
    if host.starts_with("*.") {
        (Host::Suffix(host[1..].to_string()), path)
    } else {
        (Host::Exact(host.to_string()), path)
    }
}

impl Deref for Routing {
    type Target = BTreeMap<Host, BTreeMap<Path, HandlerName>>;

    fn deref(&self) -> &BTreeMap<Host, BTreeMap<Path, HandlerName>> {
        &self.0
    }
}

impl Host {
    pub fn matches(&self, host: &str) -> bool {
        match self {
            &Host::Exact(ref h) => host == h,
            &Host::Suffix(ref h) => host.ends_with(h),
        }
    }
}

impl Deref for Host {
    type Target = String;

    fn deref(&self) -> &String {
        match *self {
            Host::Exact(ref s) => s,
            Host::Suffix(ref s) => s,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Host, Path};
    use super::parse_host_path;

    #[test]
    fn simple() {
        let s = "example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host::Exact("example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn base_host() {
        let s = "*.example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host::Suffix(".example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn invalid_base_host() {
        let s = "*example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host::Exact("*example.com".into()));
        assert!(path.is_none());

        let s = ".example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host::Exact(".example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn invalid_host() {
        // FiXME: only dot is invalid
        let s = "*.".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host::Suffix(".".into()));
        assert!(path.is_none());

        let s = "*./".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host::Suffix(".".into()));
        assert!(path.is_none());
    }

    #[test]
    fn match_host() {
        let h = Host::Exact("example.com".into());
        assert!(h.matches("example.com"));
        assert!(!h.matches(".example.com"));
        assert!(!h.matches("www.example.com"));

        let h = Host::Suffix(".example.com".into());
        assert!(!h.matches("example.com"));
        assert!(h.matches("xxx.example.com"));
        assert!(h.matches("www.example.com"));
    }
}
