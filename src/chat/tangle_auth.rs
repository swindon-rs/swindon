use std::fmt;

use rustc_serialize::json;

use base64::Base64;
use intern::SessionId;

pub struct TangleAuth<'a>(pub &'a SessionId);

impl<'a> fmt::Display for TangleAuth<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[derive(RustcEncodable)]
        struct Auth<'a> {
            user_id: &'a SessionId,
        }
        write!(f, "Tangle {}", Base64(json::encode(&Auth {
                user_id: self.0,
            }).unwrap().as_bytes()))
    }
}
