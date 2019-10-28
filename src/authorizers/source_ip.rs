use std::str::from_utf8;
use std::sync::Arc;
use std::net::IpAddr;

use tk_http::server::{Error};

use crate::config::networks::SourceIpAuthorizer;
use crate::incoming::Input;


pub fn check(cfg: &Arc<SourceIpAuthorizer>, input: &mut Input)
    -> Result<bool, Error>
{
    let forwarded = cfg.accept_forwarded_headers_from.as_ref()
        .and_then(|netw| input.config.networks.get(netw))
        .map(|netw| {
            if let Some(subnet) = netw.get_subnet(input.addr.ip()) {
                input.debug.add_allow(
                    format_args!("forwarded-from {}", subnet));
                true
            } else {
                false
            }
        })
        .unwrap_or(false);
    let ip = match (&cfg.forwarded_ip_header, forwarded) {
        (&Some(ref header), true) => {
            let mut ip = None;
            for (name, value) in input.headers.headers() {
                if name.eq_ignore_ascii_case(header) {
                    let parsed = from_utf8(value).ok()
                          .and_then(|x| x.parse::<IpAddr>().ok());
                    match parsed {
                        Some(parsed) => ip = Some(parsed),
                        None => {
                            debug!("Invalid ip {:?} from header {}",
                                String::from_utf8_lossy(value), name);
                            input.debug.set_deny(
                                "invalid-source-ip-from-header");
                            // TODO(tailhook) consider returning error
                            return Ok(false);
                        }
                    }
                }
            }
            ip.unwrap_or(input.addr.ip())
        }
        _ => input.addr.ip(),
    };
    if let Some(netw) = input.config.networks.get(&cfg.allowed_network) {
        if let Some(subnet) = netw.get_subnet(ip) {
            input.debug.add_allow(format_args!("source-ip {}", subnet));
            Ok(true)
        } else {
            input.debug.set_deny(format!("source-ip {}", ip));
            Ok(false)
        }
    } else {
        input.debug.set_deny(format!("no-network {}", cfg.allowed_network));
        Ok(false)
    }
}
