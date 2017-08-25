use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de;


pub struct FromStrVisitor<T>(&'static str, PhantomData<T>)
    where T: FromStr, T::Err: fmt::Display;

impl<T: FromStr> FromStrVisitor<T>
    where T::Err: fmt::Display
{
     pub fn new(expected: &'static str) -> FromStrVisitor<T> {
        FromStrVisitor(expected, PhantomData)
     }
}

impl<'a, T: FromStr> de::Visitor<'a> for FromStrVisitor<T>
    where T::Err: fmt::Display
{
    type Value = T;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        s.parse().map_err(|e| E::custom(e))
    }
}
