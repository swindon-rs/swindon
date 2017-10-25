use std::sync::Arc;

use ns_router::AutoName;
use quire::validate::{Enum, Scalar};


#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub enum ListenSocket {
    Tcp(String),
    // TODO(tailhook)
    // Fd(u32)
    // Unix(PathBuf)
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct Listen(Arc<Vec<ListenSocket>>);

impl Listen {
    pub fn new(vec: Vec<ListenSocket>) -> Listen {
        Listen(Arc::new(vec))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> IntoIterator for &'a Listen {
    type Item = AutoName<'a>;
    type IntoIter = ::std::iter::Map<::std::slice::Iter<'a, ListenSocket>,
                    fn(&'a ListenSocket) -> AutoName<'a>>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|x| match *x {
            ListenSocket::Tcp(ref s) => AutoName::Auto(s),
        })
    }
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("Tcp", Scalar::new())
    .default_tag("Tcp")
}
