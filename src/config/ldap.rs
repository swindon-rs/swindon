use std::collections::HashMap;

use quire::validate::{Structure, Sequence, Mapping, Scalar};

use intern::LdapUpstream;


#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct Destination {
    pub addresses: Vec<String>,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct Query {
    pub search_base: String,
    pub fetch_attribute: String,
    pub filter: String,
    pub dn_attribute_strip_base: Option<String>,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct Ldap {
    pub destination: LdapUpstream,
    pub search_base: String,
    pub login_attribute: String,
    pub password_attribute: String,
    pub login_header: Option<String>,
    pub additional_queries: HashMap<String, Query>,
}


pub fn destination_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("addresses", Sequence::new(Scalar::new()).min_length(1))
}

pub fn authorizer_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("destination", Scalar::new())
    .member("search_base", Scalar::new())
    .member("login_attribute", Scalar::new())
    .member("password_attribute", Scalar::new())
    .member("login_header", Scalar::new().optional())
    .member("additional_queries", Mapping::new(
        Scalar::new(),
        Structure::new()
        .member("search_base", Scalar::new())
        .member("fetch_attribute", Scalar::new())
        .member("filter", Scalar::new())
        .member("dn_attribute_strip_base", Scalar::new().optional())))
}
