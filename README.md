# Destruct

Destruct structs and enums into a heterogeneous list consists of a fixed set of types, 
to enable a quick implementation of combinator libraries.

## API

### `trait Destruct`

#### `type DestructType`

The destructed object type

If your struct is:
```rust
#[derive(Destruct)]
struct YourStruct {
    field: YourField,
    field2: YourField2,
}
```
Then the DestructType is:

```
DestructBegin<Fields, m>
    where Fields = DestructField<YourField, NextField, m1>
          NextField = DestructField<YourField2, End, m2>
          End = DestructEnd<m>
    where m is some generated type implementing `trait DestructMetadata`
          m1 is the metadata for `field`, implementing `trait DestructFieldMetadata + DestructMetadata`
          m2 is the metadata for `field2`, implementing `trait DestructFieldMetadata + DestructMetadata`
}
```

Here is a list of types may appear in DestructType:

- DestructBegin
- DestructField
- DestructEnd
- DestructEnumBegin
- DestructEnumVariant
- DestructEnumEnd

#### `fn destruct(self) -> Self::DestructType`

Destruct self to destruct type

#### `fn construct(d: Self::DestructType) -> Self;`

Construct self from destruct type

### Metadata

```rust
pub trait DestructMetadata {
    fn struct_name() -> &'static str;
    fn named_fields() -> bool;
}

pub trait DestructFieldMetadata: DestructMetadata + 'static {
    fn field_name() -> &'static str;
    fn field_index() -> usize;
}

pub trait DestructEnumMetadata {
    fn enum_name() -> &'static str;
}

pub trait DestructEnumVariantMetadata: DestructEnumMetadata + 'static {
    fn variant_name() -> &'static str;
    fn variant_index() -> usize;
}
```

## Example

For example, here is how to implement a parser with destruct (see `destruct-parser`):

1. Write a `Parsable` trait;
2. Implement parsers for basic types;
3. Implement parsers for six Destruct types;
4. Derive `Destruct` for your struct by adding `#[derive(Destruct)]`
5. (Optional) Implement a macro for deriving `impl<T: Destruct> Parsable`, namely `parsable!`;
6. (Optional) Implement `Parsable` for your struct by adding `#[destruct(parsable)]`

### Why do I need `parsable!` macro?

Because Rust forbids overlapping implementation of traits. Ideally what I need is the following trait implementation:

```rust
impl<T: Destruct> Parsable for T where T::DestructType: Parsable {}
```

But Rust complains:

```
upstream crates may add new impl of trait `destruct::Destruct` for type `destruct::DestructEnumBegin<_, _>` in future versions
```

So I added `#[destruct(parsable)]` to generate impls for every struct. It is equivalent to `parsable!(YourStruct)`.

```rust
#[macro_export]
macro_rules! parsable {
    ($t:ident) => {
        impl Parsable for $t {
            fn parse<R: io::Read + Clone>(read: &mut R) -> Result<Self, Error> {
                <$t as Destruct>::DestructType::parse(read).map(<$t as Destruct>::construct)
            }
        }
    };
}
```
