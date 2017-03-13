use std::collections::BTreeMap;

use config::{RouteHost, RoutePath};


/// Map host port to a route of arbitrary type
///
/// Returns destination route and relative path
pub fn route<'x, D>(host: &str, path: &'x str,
    table: &'x BTreeMap<RouteHost, BTreeMap<RoutePath, D>>)
    -> Option<(&'x D, &'x str, &'x str)>
{
    // TODO(tailhook) transform into range iteration when `btree_range` is
    // stable
    for (route_host, sub_table) in table.iter().rev() {
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
mod test {
    use config::{RouteHost, RoutePath};
    use super::route;

    #[test]
    fn route_host() {
        let table = vec![
            (RouteHost::Exact("example.com".into()), vec![
                (None, 1),
                ].into_iter().collect()),
            ].into_iter().collect();
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
        let table = vec![
            (RouteHost::Exact("example.com".into()), vec![
                (None, 1),
                ].into_iter().collect()),
            (RouteHost::Suffix(".example.com".into()), vec![
                (None, 2),
                (Some("/static".into()), 3),
                ].into_iter().collect()),
            (RouteHost::Exact("www.example.com".into()), vec![
                (Some("/static/favicon.ico".into()), 4),
                ].into_iter().collect()),
            (RouteHost::Exact("xxx.example.com".into()), vec![
                (None, 5),
                ].into_iter().collect()),
            (RouteHost::Suffix("*.aaa.example.com".into()), vec![
                (None, 6),
                ].into_iter().collect()),
            ].into_iter().collect();

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
    /*

    #[test]
    fn route_path() {
        let table = vec![
            (Route { host: "ex.com".into(), path: Some("/one".into()) }, 1),
            (Route { host: "ex.com".into(), path: None }, 0),
            (Route { host: "ex.com".into(), path: Some("/two".into()) }, 2),
            ].into_iter().collect();
        assert_eq!(route("ex.com", "/one", &table),
                   Some((&1, "")));
        assert_eq!(route("ex.com", "/one/end", &table),
                   Some((&1, "/end")));
        assert_eq!(route("ex.com", "/two", &table),
                   Some((&2, "")));
        assert_eq!(route("ex.com","/two/some", &table),
                   Some((&2, "/some")));
        assert_eq!(route("ex.com", "/three", &table),
                   Some((&0, "/three")));
        assert_eq!(route("ex.com", "/", &table),
                   Some((&0, "/")));
        assert_eq!(route("ex.org", "/one", &table), None);
        assert_eq!(route("subdomain.ex.org", "/two", &table), None);
        assert_eq!(route("example.org", "/", &table), None);
        assert_eq!(route("example.org", "/two", &table), None);
    }
    */

}
