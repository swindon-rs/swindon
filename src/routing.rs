use std::cmp::{Ordering, PartialOrd, Ord};
use std::collections::{BTreeMap, btree_map};
use std::ops::Deref;
use std::str::FromStr;

use rustc_serialize::{Decoder, Decodable};


pub type Path = Option<String>;

#[derive(Debug, PartialEq, Eq)]
pub struct RoutingTable<H>(BTreeMap<Host, BTreeMap<Path, H>>);

#[derive(Debug, PartialEq, Eq)]
pub struct Host(String);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Match<'a> {
    Word(&'a str),
    Asterisk,
}


impl<T: Decodable> Decodable for RoutingTable<T> {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_map(|mut d, n| {
            let mut rv = BTreeMap::new();
            for idx in 0..n {
                let (host, path) = d.read_map_elt_key(idx, |mut d| {
                    d.read_str().map(parse_host_path)
                })?;
                let val = d.read_map_elt_val(idx, T::decode)?;
                rv.entry(host)
                .or_insert_with(|| BTreeMap::new())
                .insert(path, val);
            }
            Ok(RoutingTable(rv))
        })
    }
}

impl<T> RoutingTable<T> {
    pub fn hosts(&self) -> btree_map::Iter<Host, BTreeMap<Path, T>> {
        self.0.iter()
    }
    #[allow(dead_code)]
    pub fn num_hosts(&self) -> usize {
        self.0.len()
    }
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

/// Map host port to a route of arbitrary type
///
/// Returns destination route and relative path
pub fn route<'x, D>(host: &str, path: &'x str,
    table: &'x RoutingTable<D>)
    -> Option<(&'x D, &'x str, &'x str)>
{
    // TODO(tailhook) transform into range iteration when `btree_range` is
    // stable
    for (route_host, sub_table) in table.hosts() {
        if route_host.matches(host) {
            for (route_path, result) in sub_table.iter().rev() {
                if path_match(&route_path, path) {
                    // Longest match is the last in reversed iteration
                    let prefix = route_path.as_ref().map(|x| &x[..]).unwrap_or("");
                    return Some((result, prefix, &path[prefix.len()..]));
                }
            }
            return None;
        }
    }
    return None;
}

fn path_match<S: AsRef<str>>(pattern: &Option<S>, value: &str) -> bool {
    if let Some(ref prefix) = *pattern {
        let prefix = prefix.as_ref();
        if value.starts_with(prefix) && (
                value.len() == prefix.len() ||
                value[prefix.len()..].starts_with("/") ||
                value[prefix.len()..].starts_with("?"))
        {
            return true;
        }
        return false;
    } else {
        return true;
    }
}


/// Returns host with trimmed whitespace and without port number if exists
pub fn parse_host(host_header: &str) -> &str {
    match host_header.find(':') {
        Some(idx) => &host_header[..idx],
        None => host_header,
    }.trim()
}

#[cfg(test)]
mod route_test {
    use super::{Host, Path};
    use super::route;
    use super::RoutingTable;

    #[test]
    fn route_host() {
        let table = RoutingTable(vec![
            ("example.com".parse().unwrap(), vec![
                (None, 1),
                ].into_iter().collect()),
            ].into_iter().collect());
        assert_eq!(route("example.com", "/hello", &table),
                   Some((&1, "", "/hello")));
        assert_eq!(route("example.com", "/", &table),
                   Some((&1, "", "/")));
        assert_eq!(route("example.org", "/hello", &table), None);
        assert_eq!(route("example.org", "/", &table), None);
    }

    #[test]
    fn route_host_suffix() {
        // Routing table
        //   example.com: 1
        //   *.example.com: 2
        //   *.example.com/static: 3
        //   www.example.com/static/favicon.ico: 4
        //   xxx.example.com: 5
        //   *.aaa.example.com: 6
        let table = RoutingTable(vec![
            ("example.com".parse().unwrap(), vec![
                (None, 1),
                ].into_iter().collect()),
            ("*.example.com".parse().unwrap(), vec![
                (None, 2),
                (Some("/static".into()), 3),
                ].into_iter().collect()),
            ("www.example.com".parse().unwrap(), vec![
                (Some("/static/favicon.ico".into()), 4),
                ].into_iter().collect()),
            ("xxx.example.com".parse().unwrap(), vec![
                (None, 5),
                ].into_iter().collect()),
            ("*.aaa.example.com".parse().unwrap(), vec![
                (None, 6),
                ].into_iter().collect()),
            ].into_iter().collect());

        assert_eq!(route("test.example.com", "/hello", &table),
                   Some((&2, "", "/hello")));
        assert_eq!(route("www.example.com", "/", &table), None);
        assert_eq!(route("www.example.com", "/static/i", &table), None);
        assert_eq!(route("www.example.com", "/static/favicon.ico", &table),
                   Some((&4, "/static/favicon.ico", "")));
        assert_eq!(route("xxx.example.com", "/hello", &table),
                   Some((&5, "", "/hello")));
        assert_eq!(route("example.org", "/", &table), None);
        assert_eq!(route("example.com", "/hello", &table),
                   Some((&1, "", "/hello")));
        assert_eq!(route("xxx.aaa.example.com", "/hello", &table),
                   Some((&6, "", "/hello")));
        assert_eq!(route("city.example.com", "/static", &table),
                   Some((&3, "/static", "")));
    }

    #[test]
    fn route_path() {
        let table = RoutingTable(vec![
            ("ex.com".parse().unwrap(), vec![
                (None , 0),
                (Some("/one".into()), 1),
                (Some("/two".into()) , 2),
                ].into_iter().collect()),
            ].into_iter().collect());
        assert_eq!(route("ex.com", "/one", &table),
                   Some((&1, "/one", "")));
        assert_eq!(route("ex.com", "/one/end", &table),
                   Some((&1, "/one", "/end")));
        assert_eq!(route("ex.com", "/two", &table),
                   Some((&2, "/two", "")));
        assert_eq!(route("ex.com","/two/some", &table),
                   Some((&2, "/two", "/some")));
        assert_eq!(route("ex.com", "/three", &table),
                   Some((&0, "", "/three")));
        assert_eq!(route("ex.com", "/", &table),
                   Some((&0, "", "/")));
        assert_eq!(route("ex.org", "/one", &table), None);
        assert_eq!(route("subdomain.ex.org", "/two", &table), None);
        assert_eq!(route("example.org", "/", &table), None);
        assert_eq!(route("example.org", "/two", &table), None);
    }

}

#[cfg(test)]
mod parse_test {
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
