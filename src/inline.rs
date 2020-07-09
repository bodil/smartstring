// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::SmartStringMode;
use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    slice::{from_raw_parts, from_raw_parts_mut},
    str::{from_utf8_unchecked, from_utf8_unchecked_mut},
};

#[cfg(target_endian = "big")]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
pub(crate) struct InlineString<Mode: SmartStringMode> {
    pub(crate) marker: u8,
    pub(crate) data: [MaybeUninit<u8>; 3 * std::mem::size_of::<usize>() - 1],
    phantom: std::marker::PhantomData<*const Mode>,
}

#[cfg(target_endian = "little")]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
pub(crate) struct InlineString<Mode: SmartStringMode> {
    pub(crate) data: [MaybeUninit<u8>; 3 * std::mem::size_of::<usize>() - 1],
    pub(crate) marker: u8,
    phantom: std::marker::PhantomData<*const Mode>,
}

impl<Mode: SmartStringMode> Clone for InlineString<Mode> {
    fn clone(&self) -> Self {
        unreachable!("InlineString should be copy!")
    }
}

impl<Mode: SmartStringMode> Copy for InlineString<Mode> {}

impl<Mode: SmartStringMode> Deref for InlineString<Mode> {
    type Target = str;
    fn deref(&self) -> &str {
        unsafe {
            let data = from_raw_parts(self.data.as_ref().as_ptr().cast(), self.len());
            from_utf8_unchecked(data)
        }
    }
}

impl<Mode: SmartStringMode> DerefMut for InlineString<Mode> {
    fn deref_mut(&mut self) -> &mut str {
        unsafe {
            let data = from_raw_parts_mut(self.data.as_mut().as_mut_ptr().cast(), self.len());
            from_utf8_unchecked_mut(data)
        }
    }
}

impl<Mode: SmartStringMode> InlineString<Mode> {
    pub(crate) fn new() -> Self {
        let mut ret = Self {
            marker: 128,
            data: [MaybeUninit::uninit(); 3 * std::mem::size_of::<usize>() - 1],
            phantom: std::marker::PhantomData,
        };
        //Are nullptr optimizations on?
        if Mode::DEALLOC || cfg!(lazy_null_pointer_optimizations) {
            //Initialize the 7 highest bytes of data
            for j in 0..(std::mem::size_of::<usize>() - 1) {
                #[cfg(target_endian = "little")]
                let j = 3 * std::mem::size_of::<usize>() - 3 - j;

                ret.data[j] = MaybeUninit::zeroed();
            }
        }
        ret
    }

    //len must be less than Mode::MAX_INLINE
    //If growing, the newly avaliable bytes should be visible
    pub(crate) unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= Mode::MAX_INLINE);
        self.marker = 128 | len as u8
    }

    pub(crate) fn len(&self) -> usize {
        let len = self.marker as usize & 127;
        debug_assert!(len <= Mode::MAX_INLINE);
        len
    }

    //Caller is responsible for keeping the utf-8 encoded string working
    pub(crate) unsafe fn as_mut_slice(&mut self) -> &mut [MaybeUninit<u8>] {
        self.data.as_mut()
    }

    //Very unsafe: Caller needs to ensure that the string stays properly encoded
    //and that the string doesn't overflow
    pub(crate) unsafe fn insert_bytes(&mut self, index: usize, bytes: &[u8]) {
        debug_assert!(self.is_char_boundary(index));
        debug_assert!(bytes.len() + self.len() <= Mode::MAX_INLINE);
        debug_assert!(std::str::from_utf8(bytes).is_ok());

        if bytes.is_empty() {
            return;
        }
        let len = self.len();
        let ptr = self.data.as_mut_ptr();
        if index != len {
            ptr.add(index + bytes.len())
                .copy_from(self.data.as_ptr().add(index), len - index);
        }
        ptr.add(index)
            .copy_from_nonoverlapping(bytes.as_ptr().cast(), bytes.len());
        self.set_len(len + bytes.len());
    }

    //Caller needs to ensure that the string stays properly encoded
    pub(crate) unsafe fn remove_bytes(&mut self, start: usize, end: usize) {
        let len = self.len();
        debug_assert!(start <= end);
        debug_assert!(end <= len);
        debug_assert!(self.is_char_boundary(start));
        debug_assert!(self.is_char_boundary(end));
        if start == end {
            return;
        }
        if end < len {
            let ptr = self.data.as_mut_ptr();
            ptr.add(start).copy_from(ptr.add(end), len - end);
        }
        self.set_len(len - (end - start));
    }
}

impl<Mode: SmartStringMode> From<&'_ [u8]> for InlineString<Mode> {
    fn from(bytes: &[u8]) -> Self {
        let len = bytes.len();
        assert!(len <= Mode::MAX_INLINE);
        assert!(std::str::from_utf8(bytes).is_ok());
        let mut out = Self::new();
        for (i, byte) in out.data.iter_mut().enumerate().take(len) {
            *byte = MaybeUninit::new(bytes[i]);
        }
        unsafe {
            out.set_len(len);
        }
        out
    }
}
