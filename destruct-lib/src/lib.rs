#[allow(unused_imports)]
#[macro_use]
extern crate destruct_derive;
#[allow(unused_imports)]
#[cfg(test)]
#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate derive_new;

use std::marker::PhantomData;

pub trait Destruct: Sized {
    /// The destructed object type
    ///
    /// If your struct is:
    /// ```rust,no-run
    /// #[derive(Destruct)]
    /// struct YourStruct {
    ///     field: YourField,
    ///     field2: YourField2,
    /// }
    /// ```
    /// Then the DestructType is:
    ///
    /// DestructBegin<Fields, m>
    ///     where Fields = DestructField<YourField, NextField, m1>
    ///           NextField = DestructField<YourField2, End, m2>
    ///           End = DestructEnd<m>
    ///     where m is some generated type implementing `trait DestructMetadata`
    ///           m1 is the metadata for `field`, implementing `trait DestructFieldMetadata`
    ///           m2 is the metadata for `field2`, implementing `trait DestructFieldMetadata`
    /// }
    type DestructType: From<Self> + Into<Self>;

    /// Destruct self to destruct type
    fn destruct(self) -> Self::DestructType;

    /// Construct self from destruct type
    fn construct(d: Self::DestructType) -> Self;
}

pub trait DestructMetadata {
    fn struct_name() -> &'static str;
    fn named_fields() -> bool;
}

#[derive(new, Debug, PartialEq, Eq)]
pub struct DestructBegin<T, M: DestructMetadata + 'static> {
    pub fields: T,
    #[new(default)]
    meta: PhantomData<&'static M>,
}

pub trait DestructFieldMetadata: DestructMetadata + 'static {
    fn field_name() -> &'static str;
    fn field_index() -> usize;
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
    pub fn field_name(&self) -> &'static str {
        M::field_name()
    }
    pub fn field_index(&self) -> usize {
        M::field_index()
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

pub trait DestructEnumMetadata {
    fn enum_name() -> &'static str;
}

#[derive(new, Debug, PartialEq, Eq)]
pub struct DestructEnumBegin<T, M: DestructEnumMetadata + 'static> {
    pub variants: T,
    #[new(default)]
    meta: PhantomData<&'static M>,
}

pub trait DestructEnumVariantMetadata: DestructEnumMetadata + 'static {
    fn variant_name() -> &'static str;
    fn variant_index() -> usize;
}

impl<T, M: DestructEnumMetadata + 'static> DestructEnumBegin<T, M> {
    pub fn enum_name() -> &'static str {
        M::enum_name()
    }
}

#[derive(new, Debug, PartialEq, Eq)]
pub enum DestructEnumVariant<H, T, M: DestructEnumVariantMetadata + 'static> {
    Head(H, #[new(default)] PhantomData<&'static M>),
    Tail(T, #[new(default)] PhantomData<&'static M>),
}

impl<H, T, M: DestructEnumVariantMetadata + 'static> DestructEnumVariant<H, T, M> {
    pub fn enum_name() -> &'static str {
        M::enum_name()
    }
    pub fn variant_name() -> &'static str {
        M::variant_name()
    }
    pub fn variant_index() -> usize {
        M::variant_index()
    }
}

#[derive(new, Debug, PartialEq, Eq)]
pub struct DestructEnumEnd<M: DestructEnumMetadata + 'static> {
    #[new(default)]
    meta: PhantomData<&'static M>,
}

impl<M: DestructEnumMetadata + 'static> DestructEnumEnd<M> {
    pub fn enum_name() -> &'static str {
        M::enum_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::ParseError::IOError;
    use std::io::Error;
    use std::io::Read;

    use crate as destruct;

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

    struct AA();

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

    #[derive(Destruct, Clone, Debug, PartialEq, Eq)]
    struct B(u8, u8);

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
        let name = d.fields.field_name();
        assert_eq!(name, "first");
        let name = d.fields.tail.field_name();
        assert_eq!(name, "second");
        let name = d.fields.tail.tail.field_name();
        assert_eq!(name, "third");
    }

    #[test]
    fn test_parse_struct() {
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

    #[test]
    fn test_parse_unnamed_struct() {
        let mut src = b"ab" as &[u8];
        let b: B = <B as Destruct>::DestructType::parse(&mut src)
            .unwrap()
            .into();
        assert_eq!(b, B(b'a', b'b'))
    }

    #[derive(Debug, Destruct, PartialEq, Eq)]
    enum TestEnum {
        A,
        B,
        C,
    }

    #[test]
    fn test_enum() {
        let e = TestEnum::construct(TestEnum::A.destruct());
        assert_eq!(e, TestEnum::A);
        let e = TestEnum::construct(TestEnum::B.destruct());
        assert_eq!(e, TestEnum::B);
    }

}
