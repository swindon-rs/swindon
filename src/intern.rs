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

