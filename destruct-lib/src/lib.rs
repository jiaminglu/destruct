pub trait Destruct: Sized {
    type DestructType: From<Self> + Into<Self>;
}

#[macro_use]
extern crate destruct_derive;

pub struct DestructEnd;

pub struct DestructBegin<T> {
    pub fields: T,
}

pub struct DestructField<H, T> {
    pub head: H,
    pub tail: T,
}

#[cfg(test)]
mod tests {
    use crate as destruct_lib;
    use super::*;

    #[derive(Destruct)]
    struct Test {
        i: i8,
        u: u8,
    }
}
