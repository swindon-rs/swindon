use std::collections::BTreeMap;

use config::Route;


/// Map host port to a route of arbitrary type
pub fn route<'x, D>(host: &str, path: &str, table: &'x BTreeMap<Route, D>)
    -> Option<&'x D>
{
    // TODO(tailhook) transform into range iteration when `btree_range` is
    // stable
    for (route, result) in table.iter().rev() {
        if route.host == host && path_match(&route.path, path) {
            // Longest match is the last in reversed iteration
            return Some(result);
        }
    }
    return None;
}

fn path_match<S: AsRef<str>>(pattern: &Option<S>, value: &str) -> bool {
    if let Some(ref prefix) = *pattern {
        let prefix = prefix.as_ref();
        if value.starts_with(prefix) && (
                value.len() == prefix.len() ||
                value[prefix.len()..].starts_with("/"))
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
            (Route { host: "example.com".into(), path: None }, 1),
            ].into_iter().collect();
        assert_eq!(route("example.com", "/hello", &table), Some(&1));
        assert_eq!(route("example.com", "/", &table), Some(&1));
        assert_eq!(route("example.org", "/hello", &table), None);
        assert_eq!(route("example.org", "/", &table), None);
    }

    #[test]
    fn route_path() {
        let table = vec![
            (Route { host: "ex.com".into(), path: Some("/one".into()) }, 1),
            (Route { host: "ex.com".into(), path: None }, 0),
            (Route { host: "ex.com".into(), path: Some("/two".into()) }, 2),
            ].into_iter().collect();
        assert_eq!(route("ex.com", "/one", &table), Some(&1));
        assert_eq!(route("ex.com", "/one/end", &table), Some(&1));
        assert_eq!(route("ex.com", "/two", &table), Some(&2));
        assert_eq!(route("ex.com", "/two/some", &table), Some(&2));
        assert_eq!(route("ex.com", "/three", &table), Some(&0));
        assert_eq!(route("ex.com", "/", &table), Some(&0));
        assert_eq!(route("ex.org", "/one", &table), None);
        assert_eq!(route("subdomain.ex.org", "/two", &table), None);
        assert_eq!(route("example.org", "/", &table), None);
        assert_eq!(route("example.org", "/two", &table), None);
    }

}
