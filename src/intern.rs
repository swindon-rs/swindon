use std::fmt;
use std::ascii::AsciiExt;
use string_intern::{Symbol, Validator};


struct UpstreamValidator;
pub type Upstream = Symbol<UpstreamValidator>;

struct HandlerValidator;
pub type HandlerName = Symbol<HandlerValidator>;

struct DiskPoolValidator;
pub type DiskPoolName = Symbol<DiskPoolValidator>;

struct SessionPoolValidator;
pub type SessionPoolName = Symbol<SessionPoolValidator>;

struct OldValidator;
pub type Atom = Symbol<OldValidator>;

struct SessionIdValidator;
pub type SessionId = Symbol<SessionIdValidator>;

struct TopicValidator;
pub type Topic = Symbol<TopicValidator>;

struct LatticeNamespaceValidator;
pub type Lattice = Symbol<LatticeNamespaceValidator>;

struct LatticeKeyValidator;
pub type LatticeKey = Symbol<LatticeKeyValidator>;

quick_error! {
    #[derive(Debug)]
    pub enum BadIdent {
        InvalidChar {
            description("invalid character in identifier")
        }
    }
}

fn valid_ident(val: &str) -> bool {
    val.chars().all(|c| c.is_ascii() &&
        (c.is_alphanumeric() || c == '-' || c == '_'))
}

fn valid_key(val: &str) -> bool {
    if val.len() == 0 {
        return false;
    }
    let first = val.chars().next().unwrap();
    return (first.is_ascii() && first.is_alphabetic() || first == '_') &&
        val.chars().all(|c| c.is_ascii() && (c.is_alphanumeric() || c == '_'))
}

fn valid_namespace(val: &str) -> bool {
    val.chars().all(|c| c.is_ascii() &&
        (c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
}

fn valid_sid(val: &str) -> bool {
    val.chars().all(|c| c.is_ascii() &&
        (c.is_alphanumeric() || c == '-' || c == '_' || c == ':'))
}

impl Validator for UpstreamValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_ident(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "upstream{:?}", value.as_ref())
    }
}

impl Validator for HandlerValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_ident(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "handler{:?}", value.as_ref())
    }
}

impl Validator for DiskPoolValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_ident(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "disk{:?}", value.as_ref())
    }
}

impl Validator for SessionPoolValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_ident(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "sessionpool{:?}", value.as_ref())
    }
}

impl Validator for OldValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_ident(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "old{:?}", value.as_ref())
    }
}

impl Validator for TopicValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_namespace(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "topic{:?}", value.as_ref())
    }
}

impl Validator for LatticeNamespaceValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_namespace(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "lattice{:?}", value.as_ref())
    }
}

impl Validator for LatticeKeyValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_key(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "key{:?}", value.as_ref())
    }
}

impl Validator for SessionIdValidator {
    type Err = BadIdent;
    fn validate_symbol(val: &str) -> Result<(), Self::Err> {
        if !valid_sid(val) {
            return Err(BadIdent::InvalidChar);
        }
        Ok(())
    }
    fn display(value: &Symbol<Self>, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "sessionid{:?}", value.as_ref())
    }
}
