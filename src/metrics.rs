use std::sync::Arc;

use libcantal::{self, Name, NameVisitor, Value, Collection, Error};
use owning_ref::OwningHandle;

use crate::runtime::Runtime;

pub use libcantal::{Counter, Integer};

pub type List = Vec<(Metric<'static>, &'static dyn Value)>;

pub struct Metric<'a>(pub &'a str, pub &'a str);

// this is not actually static, but we have no lifetime name for it
struct Wrapper(libcantal::ActiveCollection<'static>);

pub struct ActiveCollection(OwningHandle<Box<Vec<Box<dyn Collection>>>, Wrapper>);

impl<'a> Name for Metric<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        match key {
            "metric" => Some(self.1),
            "group" => Some(self.0),
            _ => None,
        }
    }
    fn visit(&self, s: &mut dyn NameVisitor) {
        s.visit_pair("group", self.0);
        s.visit_pair("metric", self.1);
    }
}

impl ::std::ops::Deref for Wrapper {
    type Target = ();
    fn deref(&self) -> &() { &() }
}

pub fn all(runtime: &Arc<Runtime>) -> Box<Vec<Box<dyn Collection>>> {
    Box::new(vec![
        Box::new(crate::incoming::metrics()),
        Box::new(crate::chat::metrics()),
        Box::new(crate::http_pools::metrics()),
        Box::new(crate::http_pools::pool_metrics(&runtime.http_pools)),
    ])
}

pub fn start(runtime: &Arc<Runtime>) -> Result<ActiveCollection, Error> {
    OwningHandle::try_new(all(runtime), |m| {
        libcantal::start(unsafe { &*m }).map(Wrapper)
    }).map(ActiveCollection)
}
