use crate::{SmartString, SmartStringMode};
use alloc::{
    boxed::Box,
    string::{String, ToString},
};
use arbitrary::{Arbitrary, Result, Unstructured};

impl<Mode: SmartStringMode> Arbitrary for SmartString<Mode>
where
    Mode: 'static,
{
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        String::arbitrary(u).map(Self::from)
    }

    fn arbitrary_take_rest(u: Unstructured<'_>) -> Result<Self> {
        String::arbitrary_take_rest(u).map(Self::from)
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        String::size_hint(depth)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.to_string().shrink().map(Self::from))
    }
}
