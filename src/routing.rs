use std::collections::BTreeMap;

use config::Route;


/// Map host port to a route of arbitrary type
///
/// Returns destination route and relative path
pub fn route<'x, D>(host: &str, path: &'x str, table: &'x BTreeMap<Route, D>)
    -> Option<(&'x D, &'x str, &'x str)>
{
    // TODO(tailhook) transform into range iteration when `btree_range` is
    // stable
    for (route, result) in table.iter().rev() {
        if host_match(host, &route) && path_match(&route.path, path) {
            // Longest match is the last in reversed iteration
            let prefix = route.path.as_ref().map(|x| &x[..]).unwrap_or("");
            return Some((result, prefix, &path[prefix.len()..]));
        }
    }
    return None;
}

fn host_match(host: &str, route: &Route) -> bool {
    if route.is_suffix {
        host.ends_with(route.host.as_str())
    } else {
        route.host == host
    }
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
    use config::Route;
    use super::route;

    #[test]
    fn route_host() {
        let table = vec![
            (Route { is_suffix: false, host: "example.com".into(), path: None }, 1),
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
        let table = vec![
            (Route { is_suffix: false, host: "example.com".into(), path: None }, 1),
            (Route { is_suffix: true, host: ".example.com".into(), path: None }, 2),
            (Route { is_suffix: true, host: ".example.com".into(),
                     path: Some("/static".into()) }, 3),
            (Route { is_suffix: false, host: "www.example.com".into(),
                     path: Some("/static/favicon.ico".into()) }, 4),
            (Route { is_suffix: false, host: "xxx.example.com".into(),
                     path: None }, 5),
            ].into_iter().collect();

        assert_eq!(route("test.example.com", "/hello", &table),
                   Some((&2, "", "/hello")));
        assert_eq!(route("www.example.com", "/", &table),
                   Some((&2, "", "/")));
        assert_eq!(route("www.example.com", "/static/i", &table),
                   Some((&3, "/static", "/i")));
        assert_eq!(route("www.example.com", "/static/favicon.ico", &table),
                   Some((&4, "/static/favicon.ico", "")));
        assert_eq!(route("xxx.example.com", "/hello", &table),
                   Some((&5, "", "/hello")));
        assert_eq!(route("example.org", "/", &table), None);
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
