use std::collections::{HashMap};

use regex::{self, RegexSet};

use intern::{HandlerName, Authorizer as AuthorizerName};
use config::{ConfigSource, Error};
use config::routing::{Host, HostPath, RouteDef};
use config::handlers::Handler::{self, StripWWWRedirect};
use config::authorizers::Authorizer;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Route {
    pub handler_name: HandlerName,
    pub handler: Handler,
    pub authorizer_name: AuthorizerName,
    pub authorizer: Authorizer,
}

#[derive(Debug)]
pub struct RoutingTable {
    set: RegexSet,
    table: Vec<(String, PathTable)>,
}

#[derive(Debug)]
pub struct PathTable {
    set: RegexSet,
    table: Vec<(String, Route)>,
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

fn inherit(to: &mut RouteDef, from: &RouteDef) {
    match (&mut to.authorizer, &from.authorizer) {
        (&mut ref mut dest @ None, &Some(ref x)) => {
            *dest = Some(x.clone());
        }
        _ => {}
    }
}
fn is_done(item: &RouteDef) -> bool {
    matches!(*item, RouteDef {
        handler: _,
        authorizer: Some(_),
    })
}
fn default() -> RouteDef {
    RouteDef {
        handler: HandlerName::from("default"),
        authorizer: None,
    }
}

impl PathTable {
    fn new(mut table: Vec<(String, Route)>) -> Result<PathTable, Error> {
        table.sort_by(|&(ref a, _), &(ref b, _)| {
            // sort by longest first, but then keep order reproducible
            b.len().cmp(&a.len()).then(a.cmp(b))
        });
        let rset = RegexSet::new(
            table.iter().map(|&(ref path, _)| {
                String::from("^") + &regex::escape(&path) + r"(?:$|/|\?|#)"
            }))?;
        Ok(PathTable {
            set: rset,
            table: table,
        })
    }
}

trait Resolver {
    fn handler(&self, &HandlerName) -> Option<Handler>;
    fn authorizer(&self, &AuthorizerName) -> Option<Authorizer>;
    fn route(&self, route: &RouteDef) -> Result<Route, Error> {
        let auth = route.authorizer.clone()
            .unwrap_or(AuthorizerName::from("default"));
        Ok(Route {
            handler: self.handler(&route.handler)
                .ok_or_else(|| Error::NoHandler(route.handler.clone()))?,
            handler_name: route.handler.clone(),
            authorizer: self.authorizer(&auth)
                .ok_or_else(|| Error::NoAuthorizer(auth.clone()))?,
            authorizer_name: auth,
        })
    }
}

impl<'a> Resolver for &'a ConfigSource {
    fn handler(&self, n: &HandlerName) -> Option<Handler> {
        self.handlers.get(n).cloned()
    }
    fn authorizer(&self, n: &AuthorizerName) -> Option<Authorizer> {
        self.authorizers.get(n).cloned()
    }
}

impl RoutingTable {
    pub fn new(src: &ConfigSource)
        -> Result<RoutingTable, Error>
    {
        RoutingTable::_create(src.routing.iter(), src)
    }
    fn _create<'x, I, R: Resolver>(iter: I, res: R)
        -> Result<RoutingTable, Error>
        where I: Iterator<Item=(&'x HostPath, &'x RouteDef)>,
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

        fn update_by_path(ndef: &mut RouteDef, path: &str,
            domain: &Domain)
        {
            for (idx, _) in path.rmatch_indices("/") {
                if is_done(ndef) {
                    break;
                }
                if idx == 0 {
                    if let Some(ref root) = domain.root {
                        inherit(ndef, root);
                    }
                } else {
                    if let Some(p) = domain.paths.get(&path[..idx]) {
                        inherit(ndef, p);
                    }
                }
            }
        }
        fn update_from_host(ndef: &mut RouteDef, host: Option<&Host>) {
            match host {
                Some(&Host {
                    star: Some(Domain { root: Some(ref r), .. }),
                    ..
                }) => {
                    inherit(ndef, r);
                }
                _ => {}
            }
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

                // TODO(tailhook) move it, maybe make a warning
                if res.handler(&rdef.handler) == Some(StripWWWRedirect)
                    && !host.starts_with("www.")
                {
                    return Err(Error::Routing(
                        format!("Host {:?} does not start with `www.` \
                            (required for StripWWWRedirect handler)", host)
                    ));
                }

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
        let mut hosts_table = Vec::new();
        for (name, host) in &table {
            let exact = if let Some(ref exact) = host.exact {
                let mut path_table = Vec::new();
                for (path, def) in &exact.paths {
                    let mut ndef = def.clone();
                    update_by_path(&mut ndef, path, &exact);
                    update_from_host(&mut ndef, Some(&host));
                    for (idx, _) in name.match_indices(".") {
                        if is_done(&ndef) {
                            break;
                        }
                        update_from_host(&mut ndef, table.get(&name[idx+1..]));
                    }
                    update_from_host(&mut ndef, table.get(""));
                    path_table.push((path.clone(), res.route(&ndef)?));
                }

                let mut ndef = exact.root.clone().unwrap_or(default());
                update_from_host(&mut ndef, Some(&host));
                for (idx, _) in name.match_indices(".") {
                    if is_done(&ndef) {
                        break;
                    }
                    update_from_host(&mut ndef, table.get(&name[idx+1..]));
                }
                update_from_host(&mut ndef, table.get(""));
                path_table.push((String::from(""), res.route(&ndef)?));

                Some(PathTable::new(path_table)?)
            } else {
                None
            };
            let star = if let Some(ref star) = host.star {
                let mut path_table = Vec::new();
                for (path, def) in &star.paths {
                    let mut ndef = def.clone();
                    update_by_path(&mut ndef, path, &star);
                    for (idx, _) in name.match_indices(".") {
                        if is_done(&ndef) {
                            break;
                        }
                        update_from_host(&mut ndef, table.get(&name[idx+1..]));
                    }
                    update_from_host(&mut ndef, table.get(""));
                    path_table.push((path.clone(), res.route(&ndef)?));
                }

                let mut ndef = star.root.clone().unwrap_or(default());
                update_from_host(&mut ndef, Some(&host));
                for (idx, _) in name.match_indices(".") {
                    if is_done(&ndef) {
                        break;
                    }
                    update_from_host(&mut ndef, table.get(&name[idx+1..]));
                }
                update_from_host(&mut ndef, table.get(""));
                path_table.push((String::from(""), res.route(&ndef)?));

                PathTable::new(path_table)?
            } else {
                // no star domain, this means there is an exact domain
                // it means subdomains of the exact domain must not match
                // higher level star domain for handler, but must match for
                // authorizer
                let mut ndef = default();
                for (idx, _) in name.match_indices(".") {
                    if is_done(&ndef) {
                        break;
                    }
                    update_from_host(&mut ndef, table.get(&name[idx+1..]));
                }
                update_from_host(&mut ndef, table.get(""));
                PathTable::new(vec![
                    (String::from(""), res.route(&ndef)?)
                ])?
            };
            hosts_table.push((name.clone(), star, exact));
        }
        hosts_table.sort_by(|&(ref a, _, _), &(ref b, _, _)| {
            b.len().cmp(&a.len()).then(a.cmp(b))
        });
        let mut real_table = Vec::new();
        let mut regex_table = Vec::new();
        for (name, star, exact) in hosts_table.into_iter() {
            match exact {
                Some(exact) => {
                    regex_table.push(
                        String::from("^") + &regex::escape(&name) + "$");
                    real_table.push((name.clone(), exact));
                    regex_table.push(
                        String::from(r"^.*\.") + &regex::escape(&name) + "$");
                    real_table.push((name, star));
                }
                None => {
                    if name == "" {
                        regex_table.push(String::from(r"^.*$"));
                    } else {
                        regex_table.push(
                            String::from(r"^(?:.*\.)?") +
                                &regex::escape(&name) + "$");
                    }
                    real_table.push((name, star));
                }
            }
        }
        let rset = RegexSet::new(regex_table.into_iter())?;
        let table = RoutingTable {
            set: rset,
            table: real_table,
        };
        Ok(table)
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
    let set = table.set.matches(host);
    if !set.matched_any() {
        return None;
    }
    let idx = set.iter().next().unwrap();
    let (_, ref sub_table) = table.table[idx];

    let set = sub_table.set.matches(path);
    if !set.matched_any() {
        return None;
    }
    let idx = set.iter().next().unwrap();
    let (ref rpath, ref route) = sub_table.table[idx];
    return Some((route, rpath, &path[rpath.len()..]));
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
    use super::{route, RoutingTable, Resolver};
    use intern::{HandlerName, Authorizer as AuthorizerName};
    use config::routing::{HostPath, RouteDef};
    use config::handlers::Handler;
    use config::authorizers::Authorizer;

    struct Fake;

    impl Resolver for Fake {
        fn handler(&self, _: &HandlerName) -> Option<Handler> {
            Some(Handler::HttpBin)
        }
        fn authorizer(&self, _: &AuthorizerName) -> Option<Authorizer> {
            Some(Authorizer::AllowAll)
        }
    }

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
        RoutingTable::_create(items.iter().map(|&(ref x, ref y)| (x, y)),
            Fake).unwrap()
    }

    pub fn route_h<'x>(host: &str, path: &'x str,
        table: &'x RoutingTable)
        -> Option<(&'x str, &'x str, &'x str)>
    {
        route(host, path, table)
        .map(|(x, p, s)| (&x.handler_name[..], p, s))
    }

    pub fn route_a<'x>(host: &str, path: &'x str,
        table: &'x RoutingTable) -> &'x str
    {
        route(host, path, table)
        .map(|(x, _, _)| &x.authorizer_name[..])
        .unwrap_or("default")
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
    fn nest_authorizer_path() {
        let table = table(vec![
            ("example.com", "1", ""),
            ("example.com/admin", "2", "admin"),
            ("example.com/admin/somewhere", "3", ""),
            ("example.com/somewhere", "4", ""),
        ]);
        assert_eq!(route_a("example.com", "/hello", &table), "default");
        assert_eq!(route_a("example.com", "/admin", &table), "admin");
        assert_eq!(route_a("example.com", "/admin/somewhere", &table),
            "admin");
        assert_eq!(route_a("example.com", "/admin/elsewhere", &table),
            "admin");
        assert_eq!(route_a("example.com", "/admin/else/where", &table),
            "admin");
        assert_eq!(route_a("example.com", "/elsewhere", &table), "default");
        assert_eq!(route_a("example.com", "/", &table), "default");
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
        assert_eq!(route_h("www.example.com", "/", &table),
                   Some(("default", "", "/")));
        assert_eq!(route_h("www.example.com", "/static/i", &table),
                   Some(("default", "", "/static/i")));
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
    fn route_suffix() {
        // Routing table
        //   localhost/static-file: 1
        let table = table(vec![
            ("localhost/static-file", "1", ""),
        ]);

        assert_eq!(route_h("localhost", "/static-file?a=b", &table),
                   Some(("1", "/static-file", "?a=b")));
        assert_eq!(route_h("localhost", "/static-file#a=b", &table),
                   Some(("1", "/static-file", "#a=b")));
        assert_eq!(route_h("localhost", "/static-file?x=1#a=b", &table),
                   Some(("1", "/static-file", "?x=1#a=b")));
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
                   Some(("default", "", "/hello")));
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
