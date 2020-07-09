// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{inline::InlineString, SmartString, SmartStringMode};
use std::ops::{Deref, DerefMut};

pub trait BoxedString: Deref<Target = str> + DerefMut + Into<String> {
    //This is unsafe when null pointer optimizations are used with LazyCompact
    //Then, it is unsound if the capacity of the string is 0
    unsafe fn from_string_unchecked(string: String) -> Self;
    fn capacity(&self) -> usize;
}

//Just a string, but the fields are in fixed order
#[cfg(target_endian = "big")]
#[repr(C)]
#[derive(Debug)]
pub struct PseudoString {
    capacity: usize,
    ptr: std::ptr::NonNull<u8>,
    size: usize,
}

#[cfg(target_endian = "little")]
#[cfg(not(feature = "lazy_null_pointer_optimizations"))]
#[repr(C)]
#[derive(Debug)]
//This seems to be the most common arrangement of std::String
//However, with lazy null pointer optimizations, this arrangement does not work
pub struct PseudoString {
    ptr: std::ptr::NonNull<u8>,
    capacity: usize,
    size: usize,
}

#[cfg(target_endian = "little")]
#[cfg(feature = "lazy_null_pointer_optimizations")]
#[repr(C)]
#[derive(Debug)]
pub struct PseudoString {
    ptr: std::ptr::NonNull<u8>,
    size: usize,
    capacity: std::num::NonZeroUsize,
}

impl Deref for PseudoString {
    type Target = str;
    fn deref(&self) -> &str {
        unsafe {
            let slice = std::slice::from_raw_parts(self.ptr.as_ptr().cast(), self.size);
            std::str::from_utf8_unchecked(slice)
        }
    }
}

impl DerefMut for PseudoString {
    fn deref_mut(&mut self) -> &mut str {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), self.size);
            std::str::from_utf8_unchecked_mut(slice)
        }
    }
}

impl From<PseudoString> for String {
    #[inline(always)]
    fn from(string: PseudoString) -> Self {
        unsafe {
            String::from_raw_parts(
                string.ptr.as_ptr(),
                string.size,
                usize::from(string.capacity),
            )
        }
    }
}

#[cfg(feature = "lazy_null_pointer_optimizations")]
unsafe fn to_capacity(size: usize) -> std::num::NonZeroUsize {
    std::num::NonZeroUsize::new_unchecked(size)
}

#[cfg(not(feature = "lazy_null_pointer_optimizations"))]
fn to_capacity(size: usize) -> usize {
    size
}

impl BoxedString for PseudoString {
    unsafe fn from_string_unchecked(mut string: String) -> Self {
        //into_raw_parts is nightly at the time of writing
        //In the future the following code should be replaced with
        //let (ptr, size, capacity) = string.into_raw_parts();
        let capacity = string.capacity();
        let bytes = string.as_mut_str();
        let ptr = bytes.as_mut_ptr();
        let size = bytes.len();
        std::mem::forget(string);

        Self {
            ptr: std::ptr::NonNull::new_unchecked(ptr),
            size,
            capacity: to_capacity(capacity),
        }
    }

    fn capacity(&self) -> usize {
        usize::from(self.capacity)
    }
}

#[derive(Debug)]
pub(crate) struct StringReference<'a, Mode: SmartStringMode> {
    referrant: &'a mut SmartString<Mode>,
    string: String,
}

impl<'a, Mode: SmartStringMode> StringReference<'a, Mode> {
    //Safety: Discriminant must be boxed
    pub(crate) unsafe fn from_smart_unchecked(smart: &'a mut SmartString<Mode>) -> Self {
        debug_assert_eq!(
            smart.discriminant(),
            crate::marker_byte::Discriminant::Boxed
        );
        let boxed: Mode::BoxedString = std::mem::transmute_copy(smart);
        let string = boxed.into();
        Self {
            referrant: smart,
            string,
        }
    }
}

impl<'a, Mode: SmartStringMode> Drop for StringReference<'a, Mode> {
    fn drop(&mut self) {
        let string = std::mem::replace(&mut self.string, String::new());
        if (Mode::DEALLOC && string.len() <= Mode::MAX_INLINE)
            || (!Mode::DEALLOC && cfg!(lazy_null_pointer_optimizations) && string.capacity() == 0)
        {
            let transmuted = (self as *mut Self).cast();
            unsafe {
                std::ptr::write(*transmuted, InlineString::<Mode>::from(string.as_bytes()));
            }
        } else {
            let transmuted = (self as *mut Self).cast();
            unsafe {
                std::ptr::write(
                    *transmuted,
                    Mode::BoxedString::from_string_unchecked(string),
                );
            }
        }
    }
}

impl<'a, Mode: SmartStringMode> Deref for StringReference<'a, Mode> {
    type Target = String;
    fn deref(&self) -> &String {
        &self.string
    }
}

impl<'a, Mode: SmartStringMode> DerefMut for StringReference<'a, Mode> {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.string
    }
}
