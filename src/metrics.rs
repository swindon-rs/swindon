use libcantal::{Name, NameVisitor, Value, Collection};

pub use libcantal::{Counter, Integer};

pub type List = Vec<(Metric, &'static Value)>;

pub struct Metric(pub &'static str, pub &'static str);

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

pub fn all() -> Vec<Box<Collection>> {
    vec![
        Box::new(::incoming::metrics()),
        Box::new(::chat::metrics()),
    ]
}
