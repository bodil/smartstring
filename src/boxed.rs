// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{marker_byte::Marker, FRAGMENT_SIZE};
use std::{
    cmp::Ordering,
    slice::from_raw_parts,
    str::{from_utf8_unchecked, Chars},
};

pub trait BoxedString {
    fn string(&self) -> &String;
    fn string_mut(&mut self) -> &mut String;
    fn into_string(self) -> String;

    fn cmp_with_str(&self, other: &str) -> Ordering;
    fn cmp_with_self(&self, other: &Self) -> Ordering;
    fn eq_with_str(&self, other: &str) -> bool;
    fn eq_with_self(&self, other: &Self) -> bool;

    fn update_fragment(&mut self) {}

    fn len(&self) -> usize {
        self.string().len()
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FragmentString {
    marker: Marker,
    fragment: [u8; FRAGMENT_SIZE],
    string: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FragmentSize(u8);

impl FragmentSize {
    fn new(bytes: usize, chars: usize) -> Self {
        Self(bytes as u8 | ((chars as u8) << 3))
    }

    #[inline(always)]
    fn as_u8(self) -> u8 {
        self.0
    }

    #[inline(always)]
    fn bytes(self) -> usize {
        (self.0 & 0x07) as usize
    }

    #[inline(always)]
    fn chars(self) -> usize {
        (self.0 >> 3) as usize
    }
}

fn make_fragment(string: &str, target: &mut [u8]) -> FragmentSize {
    let mut bytes = 0;
    let mut chars = 0;
    for c in string.chars() {
        let char_len = c.len_utf8();
        if bytes + char_len > FRAGMENT_SIZE {
            break;
        }
        c.encode_utf8(&mut target[bytes..]);
        bytes += char_len;
        chars += 1;
    }
    FragmentSize::new(bytes, chars)
}

impl FragmentString {
    fn set_fragment_size(&mut self, size: FragmentSize) {
        self.marker.set_data(size.as_u8());
    }

    #[inline(always)]
    fn fragment_size(&self) -> FragmentSize {
        FragmentSize(self.marker.data())
    }

    #[inline(always)]
    pub(crate) fn fragment_as_str(&self) -> &str {
        unsafe {
            let data = from_raw_parts(self.fragment.as_ptr(), self.fragment_size().bytes());
            from_utf8_unchecked(data)
        }
    }

    #[inline(always)]
    fn without_fragment(&self) -> &str {
        let frag_size = self.fragment_size().bytes();
        let len = self.string.len() - frag_size;
        unsafe {
            let ptr = self.string.as_str().as_ptr().add(frag_size);
            let data = from_raw_parts(ptr, len);
            from_utf8_unchecked(data)
        }
    }
}

// Returns Ok(Ordering) if we could conclusively determine ordering from the first pass.
// Err((left bytes seen, right bytes seen)) if we could not. We can skip over the bytes seen
// when doing the full comparison, these numbers are guaranteed to map to the same amount
// of `char`s.
#[inline(always)]
fn compare_fragments_the_slow_way(
    mut left: Chars<'_>,
    mut right: Chars<'_>,
) -> Result<Ordering, (usize, usize)> {
    let mut left_seen = 0;
    let mut right_seen = 0;
    while let (Some(left_char), Some(right_char)) = (left.next(), right.next()) {
        match left_char.cmp(&right_char) {
            Ordering::Equal => {}
            ordering => return Ok(ordering),
        }
        left_seen += left_char.len_utf8();
        right_seen += right_char.len_utf8();
    }
    Err((left_seen, right_seen))
}

impl BoxedString for FragmentString {
    #[inline(always)]
    fn string(&self) -> &String {
        &self.string
    }

    #[inline(always)]
    fn string_mut(&mut self) -> &mut String {
        &mut self.string
    }

    fn into_string(self) -> String {
        self.string
    }

    fn update_fragment(&mut self) {
        let size = make_fragment(&self.string, &mut self.fragment);
        self.set_fragment_size(size);
    }

    #[inline(always)]
    fn cmp_with_str(&self, other: &str) -> Ordering {
        match compare_fragments_the_slow_way(self.fragment_as_str().chars(), other.chars()) {
            Ok(ordering) => ordering,
            Err((left_seen, right_seen)) => self.string()[left_seen..].cmp(&other[right_seen..]),
        }
    }

    #[inline(always)]
    fn cmp_with_self(&self, other: &Self) -> Ordering {
        if self.fragment_size().chars() == other.fragment_size().chars() {
            // If fragments contain the exact same number of chars, we can safely compare them in one go.
            match self.fragment_as_str().cmp(other.fragment_as_str()) {
                Ordering::Equal => self.without_fragment().cmp(other.without_fragment()),
                otherwise => otherwise,
            }
        } else {
            // Otherwise, we have to do a slow char by char comparison.
            match compare_fragments_the_slow_way(
                self.fragment_as_str().chars(),
                other.fragment_as_str().chars(),
            ) {
                Ok(ordering) => ordering,
                Err((left_seen, right_seen)) => {
                    self.string()[left_seen..].cmp(&other.string()[right_seen..])
                }
            }
        }
    }

    #[inline(always)]
    fn eq_with_str(&self, other: &str) -> bool {
        if self.len() != other.len() || !other.starts_with(self.fragment_as_str()) {
            return false;
        }
        self.without_fragment() == &other[self.fragment_size().bytes()..]
    }

    #[inline(always)]
    fn eq_with_self(&self, other: &Self) -> bool {
        if self.len() != other.len() || self.fragment_as_str() != other.fragment_as_str() {
            return false;
        }
        self.without_fragment() == other.without_fragment()
    }
}

impl From<String> for FragmentString {
    fn from(string: String) -> Self {
        let mut fragment = [0; FRAGMENT_SIZE];
        let fragment_size = make_fragment(&string, &mut fragment);
        Self {
            marker: Marker::new_boxed(fragment_size.as_u8()),
            fragment,
            string,
        }
    }
}

impl BoxedString for String {
    #[inline(always)]
    fn string(&self) -> &String {
        self
    }

    #[inline(always)]
    fn string_mut(&mut self) -> &mut String {
        self
    }

    fn into_string(self) -> String {
        self
    }

    #[inline(always)]
    fn cmp_with_str(&self, other: &str) -> Ordering {
        self.as_str().cmp(other)
    }

    #[inline(always)]
    fn cmp_with_self(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }

    #[inline(always)]
    fn eq_with_str(&self, other: &str) -> bool {
        self == other
    }

    #[inline(always)]
    fn eq_with_self(&self, other: &Self) -> bool {
        self == other
    }
}
