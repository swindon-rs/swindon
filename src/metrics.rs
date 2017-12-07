use std::sync::Arc;

use libcantal::{self, Name, NameVisitor, Value, Collection, Error};
use owning_ref::OwningHandle;

use runtime::Runtime;

pub use libcantal::{Counter, Integer};

pub type List = Vec<(Metric, &'static Value)>;

pub struct Metric(pub &'static str, pub &'static str);

// this is not actually static, but we have no lifetime name for it
struct Wrapper(libcantal::ActiveCollection<'static>);

pub struct ActiveCollection(OwningHandle<Box<Vec<Box<Collection>>>, Wrapper>);

impl Name for Metric {
    fn get(&self, key: &str) -> Option<&str> {
        match key {
            "metric" => Some(self.1),
            "group" => Some(self.0),
            _ => None,
        }
    }
    fn visit(&self, s: &mut NameVisitor) {
        s.visit_pair("group", self.0);
        s.visit_pair("metric", self.1);
    }
}

impl ::std::ops::Deref for Wrapper {
    type Target = ();
    fn deref(&self) -> &() { &() }
}

pub fn all() -> Box<Vec<Box<Collection>>> {
    Box::new(vec![
        Box::new(::incoming::metrics()),
        Box::new(::chat::metrics()),
        Box::new(::http_pools::metrics()),
    ])
}

pub fn start(runtime: &Arc<Runtime>) -> Result<ActiveCollection, Error> {
    OwningHandle::try_new(all(), |m| {
        libcantal::start(unsafe { &*m }).map(Wrapper)
    }).map(ActiveCollection)
}
