use std::fmt;

use serde_json::to_string;

use base64::Base64;
use intern::SessionId;

pub struct TangleAuth<'a>(pub &'a SessionId);

impl<'a> fmt::Display for TangleAuth<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[derive(Serialize)]
        struct Auth<'a> {
            user_id: &'a SessionId,
        }
        write!(f, "Tangle {}", Base64(to_string(&Auth {
                user_id: self.0,
            }).unwrap().as_bytes()))
    }
}
