use std::net::IpAddr;

use quire::validate::{Structure, Sequence, Mapping, Scalar};
use rustc_serialize::{Decodable, Decoder};

use intern::Network;

#[derive(Debug, PartialEq, Eq)]
pub struct NetworkList {
    list: Vec<(IpAddr, u32)>,
}

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
                        if mask > 24 {
                            return Err(d.error("Mask must be 24 at max"));
                        }
                        Ok((ip, mask))
                    } else {
                        let ip = item.parse::<IpAddr>()
                            .map_err(|e| d.error(&e.to_string()))?;
                        Ok((ip, 24))
                    }
                })?);
            }
            Ok(NetworkList {
                list: result,
            })
        })
    }
}
