pub trait Destruct: Sized {
    type DestructType: From<Self> + Into<Self>;
    fn destruct(self) -> Self::DestructType;
    fn construct(d: Self::DestructType) -> Self;
}

#[allow(unused_imports)]
#[macro_use]
extern crate destruct_derive;
#[allow(unused_imports)]
#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate derive_new;

use std::marker::PhantomData;

pub trait DestructMetadata {
    fn struct_name() -> &'static str;
}

#[derive(new, Debug, PartialEq, Eq)]
pub struct DestructBegin<T, M: DestructMetadata + 'static> {
    pub fields: T,
    #[new(default)]
    meta: PhantomData<&'static M>,
}

pub trait DestructFieldMetadata: DestructMetadata + 'static {
    fn head_name() -> &'static str;
}

impl<T, M: DestructMetadata + 'static> DestructBegin<T, M> {
    pub fn struct_name(&self) -> &'static str {
        M::struct_name()
    }
}

#[derive(new, Debug, PartialEq, Eq)]
pub struct DestructField<H, T, M: DestructFieldMetadata + 'static> {
    pub head: H,
    pub tail: T,
    #[new(default)]
    meta: PhantomData<&'static M>,
}

impl<H, T, M: DestructFieldMetadata + 'static> DestructField<H, T, M> {
    pub fn struct_name(&self) -> &'static str {
        M::struct_name()
    }
    pub fn head_name(&self) -> &'static str {
        M::head_name()
    }
}

#[derive(new, Debug, PartialEq, Eq)]
pub struct DestructEnd<M: DestructMetadata + 'static> {
    #[new(default)]
    meta: PhantomData<&'static M>,
}

impl<M: DestructMetadata + 'static> DestructEnd<M> {
    pub fn struct_name(&self) -> &'static str {
        M::struct_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::ParseError::IOError;
    use std::io::Error;
    use std::io::Read;

    use crate as destruct_lib;

    trait Parser: Sized {
        type Error;

        fn parse<R: Read>(r: &mut R) -> Result<Self, Self::Error>;
    }

    #[derive(Debug, Error)]
    pub enum ParseError {
        #[error(display = "io error: {:?}", 0)]
        IOError(std::io::Error),
    }

    impl From<std::io::Error> for ParseError {
        fn from(e: Error) -> Self {
            IOError(e)
        }
    }

    /// Test for simple bincode
    impl Parser for u8 {
        type Error = ParseError;

        fn parse<R: Read>(r: &mut R) -> Result<Self, Self::Error> {
            let mut b = [0; 1];
            r.read_exact(&mut b)?;
            Ok(b[0])
        }
    }

    impl<M: DestructMetadata> Parser for DestructEnd<M> {
        type Error = ParseError;

        fn parse<R: Read>(_: &mut R) -> Result<Self, Self::Error> {
            Ok(DestructEnd::new())
        }
    }

    impl<
            H: Parser<Error = ParseError>,
            T: Parser<Error = ParseError>,
            M: DestructFieldMetadata,
        > Parser for DestructField<H, T, M>
    {
        type Error = ParseError;

        fn parse<R: Read>(r: &mut R) -> Result<Self, Self::Error> {
            Ok(DestructField::new(H::parse(r)?, T::parse(r)?))
        }
    }

    impl<Fields: Parser<Error = ParseError>, M: DestructMetadata> Parser for DestructBegin<Fields, M> {
        type Error = ParseError;

        fn parse<R: Read>(r: &mut R) -> Result<Self, Self::Error> {
            Ok(DestructBegin::new(Fields::parse(r)?))
        }
    }

    #[derive(Destruct, Clone, Debug, PartialEq, Eq)]
    struct A {
        first: u8,
        second: u8,
        third: u8,
    }

    #[test]
    fn test_meta() {
        let a = A {
            first: b'a',
            second: b'b',
            third: b'c',
        };
        let d = a.destruct();
        let name = d.struct_name();
        assert_eq!(name, "A");
        let name = d.fields.head_name();
        assert_eq!(name, "first");
        let name = d.fields.tail.head_name();
        assert_eq!(name, "second");
        let name = d.fields.tail.tail.head_name();
        assert_eq!(name, "third");
    }

    #[test]
    fn test_parser() {
        let mut src = b"abc" as &[u8];
        let a: A = <A as Destruct>::DestructType::parse(&mut src)
            .unwrap()
            .into();
        assert_eq!(
            a,
            A {
                first: b'a',
                second: b'b',
                third: b'c'
            }
        )
    }
}
