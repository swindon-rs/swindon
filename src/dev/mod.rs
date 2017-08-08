use std::str::FromStr;
use std::fmt::{self, Write};


#[derive(Clone, Debug)]
pub enum Destination {
    Http(String, String),
    Path(String),
}

#[derive(Clone, Debug)]
pub struct Route {
    subdomain: Option<String>,
    path: String,
    destination: Destination,
}


impl FromStr for Destination {
    type Err = String;
    fn from_str(data: &str) -> Result<Destination, String> {
        if data.starts_with("http://") {
            let mut pair = data[7..].splitn(2, '/');
            Ok(Destination::Http(
                String::from(pair.next().unwrap()),
                String::from(pair.next().unwrap_or("")),
            ))
        } else if data.starts_with("https://") {
            unimplemented!();
        } else {
            Ok(Destination::Path(String::from(data)))
        }
    }
}

impl FromStr for Route {
    type Err = String;
    fn from_str(data: &str) -> Result<Route, String> {
        let mut pair = data.splitn(2, '=');
        match (pair.next().unwrap(), pair.next()) {
            (dest, None) => {
                Ok(Route {
                    subdomain: None,
                    path: String::from(""),
                    destination: dest.parse()?,
                })
            }
            (pattern, Some(dest)) => {
                let mut pair = pattern.splitn(2, '/');
                let subdomain = pair.next().unwrap();
                Ok(Route {
                    subdomain: if subdomain == "" { None }
                               else { Some(String::from(subdomain)) },
                    path: String::from(pair.next().unwrap_or("")),
                    destination: dest.parse()?,
                })
            }
        }
    }
}

pub fn generate_config(port: u16, routes: &[Route], crossdomain: bool)
    -> String
{
    let mut buffer = String::new();
    _generate_config(&mut buffer, port, routes, crossdomain).unwrap();
    return buffer;
}

fn _generate_config(buf: &mut String, port: u16, routes: &[Route],
    crossdomain: bool)
    -> Result<(), fmt::Error>
{
    writeln!(buf, "listen: [127.0.0.1:{}]", port)?;
    writeln!(buf, "debug-routing: true")?;
    writeln!(buf, "debug-logging: true")?;
    writeln!(buf, "")?;
    writeln!(buf, "routing:")?;

    // Default status routes
    writeln!(buf, "  localhost/~~swindon-status/: status")?;
    writeln!(buf, "  devd.io/~~swindon-status/: status")?;

    for (idx, route) in routes.iter().enumerate() {
        match *route {
            Route { subdomain: Some(ref subdomain), ref path, .. } => {
                writeln!(buf, "  {}.devd.io/{}: h{}", subdomain, path, idx)?;
            }
            Route { subdomain: None, ref path, .. } => {
                writeln!(buf, "  localhost/{}: h{}", path, idx)?;
                writeln!(buf, "  devd.io/{}: h{}", path, idx)?;
            }
        }
    }
    writeln!(buf, "")?;
    writeln!(buf, "handlers:")?;
    for (idx, route) in routes.iter().enumerate() {
        match *route {
            Route { destination: Destination::Path(ref path), .. } => {
                writeln!(buf, "")?;
                writeln!(buf, "  h{}: !Static", idx)?;
                writeln!(buf, "    mode: relative_to_route")?;
                writeln!(buf, "    index-files: [index.html, index.htm]")?;
                writeln!(buf, "    path: {:?}", path)?;
                writeln!(buf, "    text-charset: utf-8")?;
                if crossdomain {
                    writeln!(buf, "    extra-headers:")?;
                    writeln!(buf, "      Access-Control-Allow-Origin: '*'")?;
                }
            }
            Route { destination: Destination::Http(_, ref path), .. } => {
                writeln!(buf, "")?;
                writeln!(buf, "  h{}: !Proxy", idx)?;
                writeln!(buf, "    mode: forward")?;
                writeln!(buf, "    ip-header: X-Forwarded-For")?;
                writeln!(buf, "    request-id-header: X-Request-Id")?;
                writeln!(buf, "    destination: d{}/{}", idx, path)?;
                if crossdomain {
                    writeln!(buf, "    extra-headers:")?;
                    writeln!(buf, "      Access-Control-Allow-Origin: '*'")?;
                }
            }
        }
    }
    writeln!(buf, "")?;
    writeln!(buf, "http-destinations:")?;

    // default status destinations
    writeln!(buf, "")?;
    writeln!(buf, "  status: !SelfStatus")?;

    for (idx, route) in routes.iter().enumerate() {
        match *route {
            Route { destination: Destination::Path(_), .. } => {}
            Route { destination: Destination::Http(ref host, _), .. } => {
                writeln!(buf, "")?;
                writeln!(buf, "  d{}:", idx)?;
                writeln!(buf, "    load-balancing: queue")?;
                writeln!(buf, "    addresses: [{}]", host)?;
            }
        }
    }
    Ok(())
}
