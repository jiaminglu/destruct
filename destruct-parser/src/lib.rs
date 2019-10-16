#[macro_use]
extern crate derive_new;
#[allow(unused_imports)]
#[macro_use]
extern crate destruct;

use byteorder::ReadBytesExt;
use destruct::*;
use failure::{Error, Fail};
use std::io;
use std::marker::PhantomData;

pub trait Parsable: Sized {
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error>;
}

impl Parsable for u8 {
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
        read.read_u8().map_err(Into::into)
    }
}

impl<M: DestructMetadata + 'static> Parsable for DestructEnd<M> {
    fn parse<R: io::Read + Clone>(_: &mut R) -> Result<Self, Error> {
        Ok(DestructEnd::new())
    }
}

impl<H: Parsable, T: Parsable, M: DestructFieldMetadata + 'static> Parsable
    for DestructField<H, T, M>
{
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
        Ok(DestructField::new(H::parse(read)?, T::parse(read)?))
    }
}

impl<F: Parsable, M: DestructMetadata + 'static> Parsable for DestructBegin<F, M> {
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
        Ok(DestructBegin::new(F::parse(read)?))
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Can not parse enum {}", 0)]
pub struct EnumParseError(&'static str);

impl<M: DestructEnumMetadata + 'static> Parsable for DestructEnumEnd<M> {
    fn parse<R: io::Read + Clone>(_: &mut R) -> Result<Self, Error> {
        Err(EnumParseError(M::enum_name()).into())
    }
}

impl<H: Parsable, T: Parsable, M: DestructEnumVariantMetadata + 'static> Parsable
    for DestructEnumVariant<H, T, M>
{
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
        let backup = read.clone();
        match H::parse(read) {
            Ok(r) => Ok(DestructEnumVariant::new_head(r)),
            Err(_) => {
                *read = backup;
                Ok(DestructEnumVariant::new_tail(T::parse(read)?))
            }
        }
    }
}

impl<T: Parsable, M: DestructEnumMetadata + 'static> Parsable for DestructEnumBegin<T, M> {
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
        Ok(DestructEnumBegin::new(T::parse(read)?))
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
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
        let backup = read.clone();
        let r = T::parse(read)?;
        if F::validate(&r) {
            Ok(Validated::new(r))
        } else {
            *read = backup;
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

impl<T: Parsable> Parsable for Vec<T> {
    fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
        let mut result = Vec::new();
        loop {
            let backup = read.clone();
            match T::parse(read) {
                Ok(i) => result.push(i),
                Err(_) => {
                    *read = backup;
                    return Ok(result);
                }
            }
        }
    }
}

/// Use macros to workaround overlapping impls
#[allow(unused_macros)]
macro_rules! parsable {
    ($t:ident) => {
        impl Parsable for $t {
            fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
                <$t as Destruct>::DestructType::parse(read).map(<$t as Destruct>::construct)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Destruct, PartialEq, Eq)]
    #[destruct(parsable)]
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
        let result: Test = Test::parse(&mut ab.as_ref()).unwrap();
        assert_eq!(
            result,
            Test {
                a: Validated::new(b'a'),
                b: Validated::new(b'2'),
            }
        )
    }

    #[derive(Debug, Destruct, PartialEq, Eq)]
    #[destruct(parsable)]
    enum TestEnum {
        CaseA(Validated<u8, IsAsciiLowerCase>, Validated<u8, IsAsciiDigit>),
        CaseB {
            a: Validated<u8, IsAsciiDigit>,
            b: Validated<u8, IsAsciiLowerCase>,
        },
    }

    #[test]
    fn test_parse_enum() {
        let s1 = b"a1";
        let s2 = b"aa";
        let s3 = b"1a";

        let result = TestEnum::parse(&mut s1.as_ref()).unwrap();
        assert_eq!(
            result,
            TestEnum::CaseA(Validated::new(b'a'), Validated::new(b'1'))
        );
        let result = TestEnum::parse(&mut s2.as_ref());
        assert!(result.is_err());
        let result = TestEnum::parse(&mut s3.as_ref()).unwrap();
        assert_eq!(
            result,
            TestEnum::CaseB {
                a: Validated::new(b'1'),
                b: Validated::new(b'a')
            }
        );
    }

    #[derive(Debug, Destruct, PartialEq, Eq)]
    #[destruct(parsable)]
    enum TestEnum2 {
        CaseA(Validated<u8, IsAsciiLowerCase>, Validated<u8, IsAsciiDigit>),
        CaseB(
            Validated<u8, IsAsciiLowerCase>,
            Validated<u8, IsAsciiLowerCase>,
        ),
    }

    #[test]
    fn test_parse_enum_same_initial() {
        let s1 = b"a1";
        let s2 = b"aa";
        let s3 = b"1a";

        let result = TestEnum2::parse(&mut s1.as_ref()).unwrap();
        assert_eq!(
            result,
            TestEnum2::CaseA(Validated::new(b'a'), Validated::new(b'1'))
        );
        let result = TestEnum2::parse(&mut s2.as_ref()).unwrap();
        assert_eq!(
            result,
            TestEnum2::CaseB(Validated::new(b'a'), Validated::new(b'a'))
        );
        let result: Result<TestEnum2, Error> = TestEnum2::parse(&mut s3.as_ref());
        assert!(result.is_err());
    }

    #[derive(new, Debug, Destruct, PartialEq, Eq)]
    #[destruct(parsable)]
    struct Identifier {
        head: Validated<u8, IsAsciiLowerCase>,
        tail: Vec<Validated<u8, IsAsciiDigit>>,
    }

    #[test]
    fn test_identifier() {
        let s1 = b"a1";
        let s2 = b"a12";
        let s3 = b"1a";

        let result = Identifier::parse(&mut s1.as_ref()).unwrap();
        assert_eq!(
            result,
            Identifier::new(Validated::new(b'a'), vec!(Validated::new(b'1')))
        );
        let result = Identifier::parse(&mut s2.as_ref()).unwrap();
        assert_eq!(
            result,
            Identifier::new(
                Validated::new(b'a'),
                vec!(Validated::new(b'1'), Validated::new(b'2'))
            )
        );
        let result = Identifier::parse(&mut s3.as_ref());
        assert!(result.is_err())
    }
}
