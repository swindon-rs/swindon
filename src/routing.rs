use std::collections::{HashMap, BTreeMap};
use std::str::FromStr;

use regex::{self, RegexSet};
use serde::de::{Deserializer, Deserialize};

use intern::{HandlerName, Authorizer as AuthorizerName};
use config::{ConfigSource, Error};
use config::routing::{Host, RouteDef};
use config::handlers::Handler;
use config::authorizers::Authorizer;
use config::visitors::FromStrVisitor;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Route {
    pub handler_name: HandlerName,
    pub handler: Handler,
    pub authorizer_name: AuthorizerName,
    pub authorizer: Authorizer,
}

pub type Path = Option<String>;

#[derive(Debug)]
pub struct RoutingTable {
    set: RegexSet,
    table: Vec<(Host, PathTable)>,
}

#[derive(Debug)]
pub struct PathTable {
    set: RegexSet,
    table: Vec<(Path, Route)>,
}

impl PartialEq for RoutingTable {
    fn eq(&self, other: &RoutingTable) -> bool {
        return self.table == other.table;
    }
}

impl Eq for RoutingTable {}

impl PartialEq for PathTable {
    fn eq(&self, other: &PathTable) -> bool {
        return self.table == other.table;
    }
}

impl Eq for PathTable {}

impl RoutingTable {
    pub fn new(src: &ConfigSource)
        -> Result<RoutingTable, Error>
    {
        unimplemented!();
        /*
        let mut to_insert = Vec::new();
        for host in items.keys() {
            if !host.0 {
                let pat = Host(true, host.1.clone());
                if !items.contains_key(&pat) {
                    to_insert.push(pat);
                }
            }
        }
        for host in to_insert {
            items.insert(host, BTreeMap::new());
        }
        let mut items: Vec<_> = items.into_iter().collect();
        items.sort_by(|&(ref a, _), &(ref b, _)| b.1.len().cmp(&a.1.len())
            .then_with(|| a.0.cmp(&b.0)));
        let regex = RegexSet::new(
            items.iter().map(|&(ref h, _)| {
                if h.0 && h.1 == "" {
                    String::from("^.*$")
                } else if h.0 {
                    String::from(r#"^(?:^|.*\.)"#) +
                        &regex::escape(&h.1) + "$"
                } else {
                    String::from("^") + &regex::escape(&h.1) + "$"
                }
            })
        )?;
        Ok(RoutingTable {
            set: regex,
            table: items,
        })
        */
    }
    pub fn hosts(&self) -> ::std::slice::Iter<(Host, PathTable)> {
        self.table.iter()
    }
    #[allow(dead_code)]
    pub fn num_hosts(&self) -> usize {
        self.table.len()
    }
}

/// Map host port to a route of arbitrary type
///
/// Returns destination route and relative path
pub fn route<'x>(host: &str, path: &'x str,
    table: &'x RoutingTable)
    -> Option<(&'x Route, &'x str, &'x str)>
{
    /*
    let set = table.set.matches(host);
    if !set.matched_any() {
        return None;
    }
    let idx = set.iter().next().unwrap();
    let (_, ref sub_table) = table.table[idx];

    for (route_path, result) in sub_table.iter().rev() {
        if path_match(&route_path, path) {
            // Longest match is the last in reversed iteration
            let prefix = route_path.as_ref().map(|x| &x[..]).unwrap_or("");
            return Some((result, prefix, &path[prefix.len()..]));
        }
    }
    return None;
    */
    unimplemented!();
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
    use super::route;
    use super::RoutingTable;

    #[test]
    fn route_host() {
        let table = RoutingTable::new(vec![
            ("example.com".parse().unwrap(), vec![
                (None, 1),
                ].into_iter().collect()),
            ].into_iter().collect()).unwrap();
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
        let table = RoutingTable::new(vec![
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
            ].into_iter().collect()).unwrap();

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
        assert_eq!(route("aaa.example.com", "/hello", &table),
                   Some((&6, "", "/hello")));
        assert_eq!(route("city.example.com", "/static", &table),
                   Some((&3, "/static", "")));
    }

    #[test]
    fn route_star() {
        // Routing table
        //   example.com: 1
        //   *: 2
        //   */path: 3
        let table = RoutingTable::new(vec![
            ("example.com".parse().unwrap(), vec![
                (None, 1),
                ].into_iter().collect()),
            ("*".parse().unwrap(), vec![
                (None, 2),
                (Some("/path".into()), 3),
                ].into_iter().collect()),
            ].into_iter().collect()).unwrap();


        assert_eq!(route("example.com", "/hello", &table),
                   Some((&1, "", "/hello")));
        assert_eq!(route("example.com", "/path", &table),
                   Some((&1, "", "/path")));
        assert_eq!(route("example.com", "/path/hello", &table),
                   Some((&1, "", "/path/hello")));
        assert_eq!(route("localhost", "/hello", &table),
                   Some((&2, "", "/hello")));
        assert_eq!(route("localhost", "/path/hello", &table),
                   Some((&3, "/path", "/hello")));
        assert_eq!(route("localhost", "/path", &table),
                   Some((&3, "/path", "")));
        assert_eq!(route("test.example.com", "/hello", &table),
                   None);
    }

    #[test]
    fn route_path() {
        let table = RoutingTable::new(vec![
            ("ex.com".parse().unwrap(), vec![
                (None , 0),
                (Some("/one".into()), 1),
                (Some("/two".into()) , 2),
                ].into_iter().collect()),
            ].into_iter().collect()).unwrap();
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
