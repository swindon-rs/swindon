use std::collections::{HashMap, BTreeMap};
use std::str::FromStr;

use regex::{self, RegexSet};
use serde::de::{Deserializer, Deserialize};

use intern::{HandlerName, Authorizer as AuthorizerName};
use config::{ConfigSource, Error};
use config::routing::{Host, HostPath, RouteDef};
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
        RoutingTable::_create(src.routing.iter(),
            |n| src.handlers.get(n),
            |n| src.authorizers.get(n))
    }
    fn _create<'x, I, H, A>(iter: I, get_handler: H, get_authorizer: A)
        -> Result<RoutingTable, Error>
        where I: Iterator<Item=(&'x HostPath, &'x RouteDef)>,
              H: Fn(&HandlerName) -> Option<&'x Handler>,
              A: Fn(&AuthorizerName) -> Option<&'x Authorizer>,
    {
        #[derive(Debug)]
        struct Host {
            exact: Option<Domain>,
            star: Option<Domain>,
        }
        #[derive(Debug)]
        struct Domain {
            root: Option<RouteDef>,
            paths: HashMap<String, RouteDef>,
        }

        let mut table = HashMap::new();
        for (&HostPath(Host(star, ref host), ref path), rdef) in iter {
            let mut entry = table.entry(host.clone())
                .or_insert(Host {
                    exact: None,
                    star: None,
                });
            let mut dom = if star {
                entry.star.get_or_insert(Domain {
                    root: None,
                    paths: HashMap::new(),
                })
            } else {
                entry.exact.get_or_insert(Domain {
                    root: None,
                    paths: HashMap::new(),
                })
            };
            if let Some(ref path) = *path {
                let old = dom.paths.insert(path.clone(), rdef.clone());
                if old.is_some() {
                    return Err(Error::Routing(
                        format!("Duplicate entry {}/{}", host, path)));
                }
            } else {
                if dom.root.is_some() {
                    return Err(Error::Routing(
                        format!("Duplicate entry {}/", host)));
                }
                dom.root = Some(rdef.clone());
            }
        }
        for (name, host) in &mut table {
            if let Some(ref mut host) = host.exact {
                for (path, def) in &mut host.paths {
                    println!("PATH {:?}", path);
                    for (idx, _) in path.rmatch_indices("/") {
                        println!("search {:?}", &path[..idx]);
                    }
                }
            }
            if let Some(ref mut host) = host.star {
                for (path, def) in &mut host.paths {
                    println!("PATH {:?}", path);
                    for (idx, _) in path.rmatch_indices("/") {
                        println!("search {:?}", &path[..idx]);
                    }
                }
            }
        }

        println!("TREE {:#?}", table);
        unimplemented!();
        /*
        let mut to_insert = Vec::new();
        for host in iter {
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
    use std::str::FromStr;
    use super::route;
    use super::RoutingTable;
    use intern::{HandlerName, Authorizer as AuthorizerName};
    use config::routing::{HostPath, RouteDef};
    use config::handlers::Handler;
    use config::authorizers::Authorizer;


    fn table(table: Vec<(&'static str, &'static str, &'static str)>)
        -> RoutingTable
    {
        let items = table.into_iter().map(|(r, h, a)| {
            (HostPath::from_str(r).unwrap(), RouteDef {
                handler: HandlerName::from(h),
                authorizer: if a == "" { None }
                    else { Some(AuthorizerName::from(a)) }
            })
        }).collect::<Vec<_>>();
        let h = Handler::HttpBin;
        let a = Authorizer::AllowAll;
        RoutingTable::_create(items.iter().map(|&(ref x, ref y)| (x, y)),
            |_| Some(&h), |_| Some(&a)).unwrap()
    }

    pub fn route_h<'x>(host: &str, path: &'x str,
        table: &'x RoutingTable)
        -> Option<(&'x str, &'x str, &'x str)>
    {
        route(host, path, table)
        .map(|(x, p, s)| (&x.handler_name[..], p, s))
    }

    pub fn route_a<'x>(host: &str, path: &'x str,
        table: &'x RoutingTable) -> Option<&'x str>
    {
        route(host, path, table)
        .map(|(x, _, _)| &x.authorizer_name[..])
    }

    #[test]
    fn route_host() {
        let table = table(vec![
            ("example.com", "1", "")
        ]);
        assert_eq!(route_h("example.com", "/hello", &table),
                   Some(("1", "", "/hello")));
        assert_eq!(route_h("example.com", "/", &table),
                   Some(("1", "", "/")));
        assert_eq!(route_h("example.org", "/hello", &table), None);
        assert_eq!(route_h("example.org", "/", &table), None);
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
        let table = table(vec![
            ("example.com", "1", ""),
            ("*.example.com", "2", ""),
            ("*.example.com/static", "3", ""),
            ("www.example.com/static/favicon.ico", "4", ""),
            ("xxx.example.com", "5", ""),
            ("*.aaa.example.com", "6", ""),
        ]);

        assert_eq!(route_h("test.example.com", "/hello", &table),
                   Some(("2", "", "/hello")));
        assert_eq!(route_h("www.example.com", "/", &table), None);
        assert_eq!(route_h("www.example.com", "/static/i", &table), None);
        assert_eq!(route_h("www.example.com", "/static/favicon.ico", &table),
                   Some(("4", "/static/favicon.ico", "")));
        assert_eq!(route_h("xxx.example.com", "/hello", &table),
                   Some(("5", "", "/hello")));
        assert_eq!(route_h("example.org", "/", &table), None);
        assert_eq!(route_h("example.com", "/hello", &table),
                   Some(("1", "", "/hello")));
        assert_eq!(route_h("xxx.aaa.example.com", "/hello", &table),
                   Some(("6", "", "/hello")));
        assert_eq!(route_h("aaa.example.com", "/hello", &table),
                   Some(("6", "", "/hello")));
        assert_eq!(route_h("city.example.com", "/static", &table),
                   Some(("3", "/static", "")));
    }

    #[test]
    fn route_star() {
        // Routing table
        //   example.com: 1
        //   *: 2
        //   * /path: 3
        let table = table(vec![
            ("example.com", "1", ""),
            ("*", "2", ""),
            ("*/path", "3", ""),
        ]);

        assert_eq!(route_h("example.com", "/hello", &table),
                   Some(("1", "", "/hello")));
        assert_eq!(route_h("example.com", "/path", &table),
                   Some(("1", "", "/path")));
        assert_eq!(route_h("example.com", "/path/hello", &table),
                   Some(("1", "", "/path/hello")));
        assert_eq!(route_h("localhost", "/hello", &table),
                   Some(("2", "", "/hello")));
        assert_eq!(route_h("localhost", "/path/hello", &table),
                   Some(("3", "/path", "/hello")));
        assert_eq!(route_h("localhost", "/path", &table),
                   Some(("3", "/path", "")));
        assert_eq!(route_h("test.example.com", "/hello", &table),
                   None);
    }

    #[test]
    fn route_path() {
        let table = table(vec![
            ("ex.com", "0", ""),
            ("ex.com/one", "1", ""),
            ("ex.com/two", "2", ""),
        ]);
        assert_eq!(route_h("ex.com", "/one", &table),
                   Some(("1", "/one", "")));
        assert_eq!(route_h("ex.com", "/one/end", &table),
                   Some(("1", "/one", "/end")));
        assert_eq!(route_h("ex.com", "/two", &table),
                   Some(("2", "/two", "")));
        assert_eq!(route_h("ex.com","/two/some", &table),
                   Some(("2", "/two", "/some")));
        assert_eq!(route_h("ex.com", "/three", &table),
                   Some(("0", "", "/three")));
        assert_eq!(route_h("ex.com", "/", &table),
                   Some(("0", "", "/")));
        assert_eq!(route_h("ex.org", "/one", &table), None);
        assert_eq!(route_h("subdomain.ex.org", "/two", &table), None);
        assert_eq!(route_h("example.org", "/", &table), None);
        assert_eq!(route_h("example.org", "/two", &table), None);
    }

}
