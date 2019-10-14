#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate destruct_derive;

use byteorder::ReadBytesExt;
use destruct_lib::*;
use failure::{Error, Fail};
use std::io;
use std::marker::PhantomData;

pub trait ParserRead: io::Read {
    fn take_while<F>(&mut self, f: F) -> &[u8]
    where
        F: Fn(u8) -> bool;
}

impl<'a> ParserRead for &'a [u8] {
    fn take_while<F>(&mut self, f: F) -> &[u8]
    where
        F: Fn(u8) -> bool,
    {
        let mut i = 0;
        while f(self[i]) {
            i += 1;
        }
        &self[..i]
    }
}

pub trait Parsable: Sized {
    fn parse<R: ParserRead>(read: &mut R) -> Result<Self, Error>;
}

impl Parsable for u8 {
    fn parse<R: ParserRead>(read: &mut R) -> Result<Self, Error> {
        read.read_u8().map_err(Into::into)
    }
}

impl<M: DestructMetadata + 'static> Parsable for DestructEnd<M> {
    fn parse<R: ParserRead>(_: &mut R) -> Result<Self, Error> {
        Ok(DestructEnd::new())
    }
}

impl<H: Parsable, T: Parsable, M: DestructFieldMetadata + 'static> Parsable
    for DestructField<H, T, M>
{
    fn parse<R: ParserRead>(read: &mut R) -> Result<Self, Error> {
        Ok(DestructField::new(H::parse(read)?, T::parse(read)?))
    }
}

impl<F: Parsable, M: DestructMetadata + 'static> Parsable for DestructBegin<F, M> {
    fn parse<R: ParserRead>(read: &mut R) -> Result<Self, Error> {
        Ok(DestructBegin::new(F::parse(read)?))
    }
}

pub trait Validator<T: Sized> {
    fn validate(value: &T) -> bool;
    fn description() -> &'static str;
}

#[derive(new, Debug, PartialEq, Eq)]
pub struct Validated<T, F: Validator<T> + 'static> {
    value: T,
    #[new(default)]
    validator: PhantomData<&'static F>,
}

#[derive(Debug, Fail)]
#[fail(display = "Can not validate {}", 0)]
pub struct ValidateError(&'static str);

impl<T: Parsable, F: Validator<T>> Parsable for Validated<T, F> {
    fn parse<R: ParserRead>(read: &mut R) -> Result<Self, Error> {
        let r = T::parse(read)?;
        if F::validate(&r) {
            Ok(Validated::new(r))
        } else {
            Err(ValidateError(F::description()).into())
        }
    }
}

macro_rules! define_validator {
    ($name:ident, |$value:ident : &$t:ty| $e:expr) => {
        #[derive(Debug, PartialEq, Eq)]
        pub struct $name;

        impl Validator<$t> for $name {
            fn validate($value: &$t) -> bool {
                $e
            }
            fn description() -> &'static str {
                stringify!($name)
            }
        }
    };
}

define_validator!(IsAsciiDigit, |value: &u8| *value >= b'0' && *value <= b'9');
define_validator!(IsAsciiLowerCase, |value: &u8| *value >= b'a'
    && *value <= b'z');
define_validator!(IsAsciiUpperCase, |value: &u8| *value >= b'A'
    && *value <= b'Z');

pub fn parse_struct<T: Destruct, R: ParserRead>(r: &mut R) -> Result<T, Error>
where
    T::DestructType: Parsable,
{
    T::DestructType::parse(r).map(T::construct)
}

pub fn parse<T: Parsable, R: ParserRead>(r: &mut R) -> Result<T, Error> {
    T::parse(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Destruct, PartialEq, Eq)]
    struct Test {
        a: Validated<u8, IsAsciiLowerCase>,
        b: Validated<u8, IsAsciiDigit>,
    }

    #[test]
    fn test_simple() {
        let s = b"a";
        let d = b"2";
        let result = Validated::<u8, IsAsciiDigit>::parse(&mut &s[..]);
        assert!(result.is_err());
        let result = Validated::<u8, IsAsciiDigit>::parse(&mut &d[..]);
        assert_eq!(result.unwrap(), Validated::new(b'2'));
    }

    #[test]
    fn test_struct() {
        let ab = b"a2";
        let result: Test = parse_struct(&mut ab.as_ref()).unwrap();
        assert_eq!(
            result,
            Test {
                a: Validated::new(b'a'),
                b: Validated::new(b'2'),
            }
        )
    }

}
