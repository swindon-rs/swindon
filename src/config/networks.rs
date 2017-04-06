use std::fmt;
use std::net::IpAddr;

use quire::validate::{Structure, Sequence, Scalar};
use rustc_serialize::{Decodable, Decoder};

use intern::Network;

#[derive(Debug, PartialEq, Eq)]
pub struct NetworkList {
    list: Vec<Subnet>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subnet(IpAddr, u32);

#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub struct SourceIpAuthorizer {
    pub allowed_network: Network,
    pub forwarded_ip_header: Option<String>,
    pub accept_forwarded_headers_from: Option<Network>,
}

pub fn source_ip_authorizer_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("allowed_network", Scalar::new())
    .member("forwarded_ip_header", Scalar::new().optional())
    .member("accept_forwarded_headers_from", Scalar::new().optional())
}

pub fn validator<'x>() -> Sequence<'x> {
    Sequence::new(Scalar::new())
}

impl Decodable for NetworkList {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_seq(|d, num| {
            let mut result = Vec::new();
            for i in 0..num {
                result.push(d.read_seq_elt(i, |d| {
                    let item = d.read_str()?;
                    if let Some(pos) = item.find('/') {
                        let ip = item[..pos].parse::<IpAddr>()
                            .map_err(|e| d.error(&e.to_string()))?;
                        let mask = item[pos+1..].parse::<u32>()
                            .map_err(|e| d.error(&e.to_string()))?;
                        let max_mask = match ip {
                            IpAddr::V4(_) => 24,
                            IpAddr::V6(_) => 128,
                        };
                        if mask % 8 != 0 {
                            return Err(d.error("Subnet mask must \
                                be multiple of eight"));
                        }
                        if mask > max_mask {
                            return Err(d.error(
                                &format!("Mask must be {} at max", max_mask)));
                        }
                        Ok(Subnet(ip, mask))
                    } else {
                        let ip = item.parse::<IpAddr>()
                            .map_err(|e| d.error(&e.to_string()))?;
                        match ip {
                            IpAddr::V4(_) => Ok(Subnet(ip, 24)),
                            IpAddr::V6(_) => Ok(Subnet(ip, 128)),
                        }
                    }
                })?);
            }
            Ok(NetworkList {
                list: result,
            })
        })
    }
}

impl fmt::Display for Subnet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

impl NetworkList {
    pub fn get_subnet(&self, ip: IpAddr) -> Option<&Subnet> {
        for item in &self.list {
            match (ip, item) {
                (IpAddr::V4(my), &Subnet(IpAddr::V4(net), msk)) => {
                    let bytes = (msk / 8) as usize;
                    if my.octets()[..bytes] == net.octets()[..bytes] {
                        return Some(item);
                    }
                }
                (IpAddr::V6(my), &Subnet(IpAddr::V6(net), msk)) => {
                    let bytes = (msk / 8) as usize;
                    if my.octets()[..bytes] == net.octets()[..bytes] {
                        return Some(item);
                    }
                }
                _ => {}
            }
        }
        return None;
    }
}
