use crate::{casts::StringCastMut, SmartString, SmartStringMode};
use std::{
    fmt::{Debug, Error, Formatter},
    iter::FusedIterator,
    ops::{Bound, RangeBounds},
};

/// A draining iterator for a [`SmartString`][SmartString].
///
/// [SmartString]: struct.SmartString.html
pub struct Drain<'a, Mode: SmartStringMode> {
    string: *mut SmartString<Mode>,
    iterator: std::str::Chars<'a>,
    start: usize,
    end: usize,
}

impl<'a, Mode: SmartStringMode> Drain<'a, Mode> {
    /// Creates a new draining iterator for a [`SmartString`][SmartString].
    ///
    /// [SmartString]: struct.SmartString.html
    pub fn new<R: RangeBounds<usize>>(from: &'a mut SmartString<Mode>, range: R) -> Self {
        let start = match range.start_bound() {
            Bound::Included(x) => *x,
            Bound::Unbounded => 0,
            Bound::Excluded(x) => x.checked_add(1).unwrap(),
        };
        let end = match range.end_bound() {
            Bound::Excluded(x) => *x,
            Bound::Unbounded => from.len(),
            Bound::Included(x) => x.checked_add(1).unwrap(),
        };
        Self {
            string: from,
            iterator: from.as_str()[start..end].chars(),
            start,
            end,
        }
    }
}

impl<'a, Mode: SmartStringMode> Iterator for Drain<'a, Mode> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}

impl<'a, Mode> DoubleEndedIterator for Drain<'a, Mode>
where
    Mode: SmartStringMode,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iterator.next_back()
    }
}

impl<'a, Mode> FusedIterator for Drain<'a, Mode> where Mode: SmartStringMode {}

impl<'a, Mode> Debug for Drain<'a, Mode>
where
    Mode: SmartStringMode,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.pad("Drain { ... }")
    }
}

impl<'a, Mode> Drop for Drain<'a, Mode>
where
    Mode: SmartStringMode,
{
    fn drop(&mut self) {
        //We must first replace the iterator with a dummy one, so it won't dangle
        self.iterator = "".chars();
        //Now we can safely clear the string
        unsafe {
            match (*self.string).cast_mut() {
                StringCastMut::Boxed(mut string) => {
                    string.drain(self.start..self.end);
                }
                StringCastMut::Inline(string) => string.remove_bytes(self.start, self.end),
            }
        }
    }
}
