pub trait Destruct: Sized {
    type DestructType: From<Self> + Into<Self>;
    fn destruct(self) -> Self::DestructType;
    fn construct(d: Self::DestructType) -> Self;
}

#[macro_use]
extern crate destruct_derive;
#[macro_use]
extern crate err_derive;

#[derive(Debug, PartialEq, Eq)]
pub struct DestructEnd;

#[derive(Debug, PartialEq, Eq)]
pub struct DestructBegin<T> {
    pub fields: T,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DestructField<H, T> {
    pub head: H,
    pub tail: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as destruct_lib;

    #[derive(Destruct, Clone, Debug, PartialEq, Eq)]
    struct Test {
        i: i8,
        u: u8,
    }

    #[test]
    fn test() {
        let test = Test { i: 0, u: 1 };
        let d: DestructBegin<DestructField<i8, DestructField<u8, DestructEnd>>> = DestructBegin{fields: DestructField{head:0, tail:DestructField{head:1, tail:DestructEnd}}};
        assert_eq!(d, test.clone().destruct());
        assert_eq!(Test::construct(d), test);
    }

    use super::*;
    use crate::tests::ParseError::IOError;
    use destruct_lib::*;
    use std::io::Error;
    use std::io::Read;

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

    impl Parser for DestructEnd {
        type Error = ParseError;

        fn parse<R: Read>(_: &mut R) -> Result<Self, Self::Error> {
            Ok(DestructEnd)
        }
    }

    impl<H: Parser<Error = ParseError>, T: Parser<Error = ParseError>> Parser for DestructField<H, T> {
        type Error = ParseError;

        fn parse<R: Read>(r: &mut R) -> Result<Self, Self::Error> {
            Ok(DestructField {
                head: H::parse(r)?,
                tail: T::parse(r)?,
            })
        }
    }

    impl<Fields: Parser<Error = ParseError>> Parser for DestructBegin<Fields> {
        type Error = ParseError;

        fn parse<R: Read>(r: &mut R) -> Result<Self, Self::Error> {
            Ok(DestructBegin {
                fields: Fields::parse(r)?,
            })
        }
    }

    #[derive(Destruct, Clone, Debug, PartialEq, Eq)]
    struct A {
        first: u8,
        second: u8,
        third: u8,
    }

    #[test]
    fn test_parser() {
        let mut src = b"abc" as &[u8];
        let a: A = <A as Destruct>::DestructType::parse(&mut src).unwrap().into();
        assert_eq!(a, A {first: b'a', second: b'b', third: b'c'})
    }
}
