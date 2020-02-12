use std::sync::Arc;
use std::fmt;

use crate::runtime::Runtime;

// Mostly follow libcantal style with some special cases
// with prometheus metric labels and value formatting

pub fn all(runtime: &Arc<Runtime>) -> Vec<Box<dyn Collection>> {
    vec![
        Box::new(crate::incoming::prometheus_metrics()),
        Box::new(crate::chat::prometheus_metrics()),
        Box::new(crate::http_pools::prometheus_metrics()),
        Box::new(crate::http_pools::pool_metrics(&runtime.http_pools)),
    ]
}


pub trait Value: fmt::Display {
    fn type_name(&self) -> &'static str;
}

impl Value for libcantal::Counter {
    fn type_name(&self) -> &'static str {
        "counter"
    }
}
impl Value for libcantal::Integer {
    fn type_name(&self) -> &'static str {
        "gauge"
    }
}

pub type List<'a> = Vec<(&'a str, &'a [(&'a str, &'a str)], &'a dyn Value)>;

pub trait Visit {
    fn metric(&mut self, name: &'static str, labels: &[(&str, &str)], value: &dyn Value);

    fn info_metric(&mut self, name: &'static str, labels: &[(&str, &str)]) {
        self.metric(name, labels, &One)
    }
}

struct One;

impl Value for One {
    fn type_name(&self) -> &'static str {
        "gauge"
    }
}
impl fmt::Display for One {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "1.0")
    }
}


pub trait Collection {

    fn visit(&self, visitor: &mut dyn Visit);
}

impl Collection for List<'static> {
    fn visit(&self, visitor: &mut dyn Visit) {
        for &(m, l, v) in self.iter() {
            visitor.metric(m, l, v)
        }
    }
}
impl Collection for Vec<Box<dyn Collection>> {
    fn visit(&self, visitor: &mut dyn Visit) {
        for sub in self.iter() {
            sub.visit(visitor)
        }
    }
}
