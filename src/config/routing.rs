use std::str::FromStr;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::cmp::{Ordering, PartialOrd, Ord};

use intern::HandlerName;
use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Mapping, Scalar};


pub type Path = Option<String>;

#[derive(Debug, PartialEq, Eq)]
pub struct Host(String);

#[derive(Debug, PartialEq, Eq)]
pub struct Routing(pub BTreeMap<Host, BTreeMap<Path, HandlerName>>);

pub fn validator<'x>() -> Mapping<'x> {
    Mapping::new(Scalar::new(), Scalar::new())
}


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Match<'a> {
    Word(&'a str),
    Asterisk,
}

impl<'a> Match<'a> {
    fn new(val: &'a str) -> Match<'a> {
        if val == "*" {
            Match::Asterisk
        } else {
            Match::Word(val)
        }
    }
}

impl Ord for Host {
    fn cmp(&self, other: &Host) -> Ordering {
        let a = self.0.split('.').rev().map(Match::new);
        let b = other.0.split('.').rev().map(Match::new);
        a.cmp(b)
    }
}

impl PartialOrd for Host {
    fn partial_cmp(&self, other: &Host) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Host {
    pub fn matches(&self, host: &str) -> bool {
        let h = self.0.as_str();
        if h.starts_with("*.") {
            host.ends_with(&h[1..])
        } else {
            host == h
        }
    }
}

impl FromStr for Host {
    type Err = ();

    fn from_str(val: &str) -> Result<Host, ()> {
        Ok(Host(val.to_string()))
    }
}

impl Deref for Host {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
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

impl Deref for Routing {
    type Target = BTreeMap<Host, BTreeMap<Path, HandlerName>>;

    fn deref(&self) -> &BTreeMap<Host, BTreeMap<Path, HandlerName>> {
        &self.0
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
    (host.parse().unwrap(), path)
}

#[cfg(test)]
mod test {
    use super::{Host, Path};
    use super::parse_host_path;

    #[test]
    fn simple() {
        let s = "example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host("example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn base_host() {
        let s = "*.example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host("*.example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn invalid_base_host() {
        let s = "*example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host("*example.com".into()));
        assert!(path.is_none());

        let s = ".example.com".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host(".example.com".into()));
        assert!(path.is_none());
    }

    #[test]
    fn invalid_host() {
        // FiXME: only dot is invalid
        let s = "*.".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host("*.".into()));
        assert!(path.is_none());

        let s = "*./".to_string();
        let (host, path) = parse_host_path(s);
        assert_eq!(host, Host("*.".into()));
        assert!(path.is_none());
    }

    #[test]
    fn match_host() {
        let h = Host("example.com".into());
        assert!(h.matches("example.com"));
        assert!(!h.matches(".example.com"));
        assert!(!h.matches("www.example.com"));

        let h = Host("*.example.com".into());
        assert!(!h.matches("example.com"));
        assert!(h.matches("xxx.example.com"));
        assert!(h.matches("www.example.com"));
    }

    #[test]
    fn ordering() {
        let mut ordered: Vec<Host> = vec![
            "aaa".parse().unwrap(),
            "*.bbb".parse().unwrap(),
            "*.aaa.bbb".parse().unwrap(),
            "*.zzz.bbb".parse().unwrap(),
            "aaa.zzz".parse().unwrap(),
        ];
        ordered.sort();
        assert_eq!(ordered, [
            Host("aaa".into()),
            Host("*.aaa.bbb".into()),
            Host("*.zzz.bbb".into()),
            Host("*.bbb".into()),
            Host("aaa.zzz".into()),
        ]);
        ordered.reverse();
        assert_eq!(ordered, [
            Host("aaa.zzz".into()),
            Host("*.bbb".into()),
            Host("*.zzz.bbb".into()),
            Host("*.aaa.bbb".into()),
            Host("aaa".into()),
        ]);
    }

}
