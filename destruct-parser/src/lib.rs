#[macro_use]
extern crate err_derive;

use std::io::Read;

trait ParserRead: Read {
    fn read_while<F>(&mut self, f: F) -> &[u8] where F: FnMut(u8) -> bool;
}

trait Parser: Sized {
    type Error;

    fn parse<R: ParserRead>(r: &mut R) -> Result<Self, Self::Error>;
}

#[cfg(test)]
mod tests {

    use super::*;
    use destruct_lib::*;
    use crate::tests::ParseError::IOError;
    use std::io::Error;

    #[derive(Debug, Error)]
    pub enum ParseError {
        #[error(display = "io error: {:?}", 0)]
        IOError(std::io::Error)
    }

    impl From<std::io::Error> for ParseError {
        fn from(e: Error) -> Self {
            IOError(e)
        }
    }

    /// Test for simple bincode
    impl Parser for u8 {
        type Error = ParseError;

        fn parse<R: ParserRead>(r: &mut R) -> Result<Self, Self::Error> {
            let mut b = [0; 1];
            r.read_exact(&mut b)?;
            Ok(b[0])
        }
    }

    impl Parser for DestructEnd {
        type Error = ParseError;

        fn parse<R: ParserRead>(r: &mut R) -> Result<Self, Self::Error> {
            Ok(DestructEnd)
        }
    }

    impl <H: Parser<Error=ParseError>, T: Parser<Error=ParseError>> Parser for DestructField<H, T> {
        type Error = ParseError;

        fn parse<R: ParserRead>(r: &mut R) -> Result<Self, Self::Error> {
            Ok(
                DestructField {
                    head: H::parse(r)?,
                    tail: T::parse(r)?,
                }
            )
        }
    }

    impl <Fields: Parser<Error=ParseError>> Parser for DestructBegin<Fields> {
        type Error = ParseError;

        fn parse<R: ParserRead>(r: &mut R) -> Result<Self, Self::Error> {
            Ok(
                DestructBegin {
                    fields: Fields::parse(r)?,
                }
            )
        }
    }

//    impl <T: Parser, FT: Destruct<DestructType=T>> Parser for FT {
//        type Error = T::Error;
//
//        fn parse<R: ParserRead>(r: &mut R) -> Result<Self, Self::Error> {
//            Ok(T::DestructType::parse(r)?.into())
//        }
//    }

}
