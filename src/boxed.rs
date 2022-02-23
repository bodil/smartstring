// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use alloc::{alloc::Layout, string::String};
use core::{
    mem::forget,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::{ops::GenericString, MAX_INLINE};

#[cfg(not(endian = "big"))]
#[repr(C)]
pub(crate) struct BoxedString {
    ptr: NonNull<u8>,
    cap: usize,
    len: usize,
}

#[cfg(endian = "big")]
#[repr(C)]
pub(crate) struct BoxedString {
    length: usize,
    cap: usize,
    ptr: NunNull<u8>,
}

impl GenericString for BoxedString {
    fn set_size(&mut self, size: usize) {
        self.len = size;
        debug_assert!(self.len <= self.cap);
    }

    fn as_mut_capacity_slice(&mut self) -> &mut [u8] {
        #[allow(unsafe_code)]
        unsafe {
            core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.capacity())
        }
    }
}

impl BoxedString {
    const MINIMAL_CAPACITY: usize = MAX_INLINE * 2;

    fn layout_for(cap: usize) -> Layout {
        let layout = Layout::array::<u8>(cap).unwrap();
        assert!(
            layout.size() <= isize::MAX as usize,
            "allocation too large!"
        );
        layout
    }

    fn alloc(cap: usize) -> NonNull<u8> {
        let layout = Self::layout_for(cap);
        #[allow(unsafe_code)]
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => alloc::alloc::handle_alloc_error(layout),
        }
    }

    fn realloc(&mut self, cap: usize) {
        let layout = Self::layout_for(cap);
        let old_layout = Self::layout_for(self.cap);
        let old_ptr = self.ptr.as_ptr();
        #[allow(unsafe_code)]
        let ptr = unsafe { alloc::alloc::realloc(old_ptr, old_layout, layout.size()) };
        self.ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => alloc::alloc::handle_alloc_error(layout),
        };
        self.cap = cap;
    }

    pub(crate) fn ensure_capacity(&mut self, target_cap: usize) {
        let mut cap = self.cap;
        while cap < target_cap {
            cap *= 2;
        }
        self.realloc(cap)
    }

    pub(crate) fn new(cap: usize) -> Self {
        let cap = cap.max(Self::MINIMAL_CAPACITY);
        Self {
            cap,
            len: 0,
            ptr: Self::alloc(cap),
        }
    }

    pub(crate) fn from_str(cap: usize, src: &str) -> Self {
        let mut out = Self::new(cap);
        out.len = src.len();
        out.as_mut_capacity_slice()[..src.len()].copy_from_slice(src.as_bytes());
        out
    }

    pub(crate) fn capacity(&self) -> usize {
        self.cap
    }

    pub(crate) fn shrink_to_fit(&mut self) {
        self.realloc(self.len);
    }
}

impl Drop for BoxedString {
    fn drop(&mut self) {
        #[allow(unsafe_code)]
        unsafe {
            alloc::alloc::dealloc(self.ptr.as_ptr(), Self::layout_for(self.cap))
        }
    }
}

impl Clone for BoxedString {
    fn clone(&self) -> Self {
        Self::from_str(self.capacity(), self.deref())
    }
}

impl Deref for BoxedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        #[allow(unsafe_code)]
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(self.ptr.as_ptr(), self.len))
        }
    }
}

impl DerefMut for BoxedString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[allow(unsafe_code)]
        unsafe {
            core::str::from_utf8_unchecked_mut(core::slice::from_raw_parts_mut(
                self.ptr.as_ptr(),
                self.len,
            ))
        }
    }
}

impl From<String> for BoxedString {
    fn from(mut s: String) -> Self {
        if s.is_empty() {
            Self::new(s.capacity())
        } else {
            // TODO: Use String::into_raw_parts when stabilised, meanwhile let's get unsafe
            let len = s.len();
            let cap = s.capacity();
            #[allow(unsafe_code)]
            let ptr = unsafe { NonNull::new_unchecked(s.as_mut_ptr()) };
            forget(s);
            Self { cap, len, ptr }
        }
    }
}

impl Into<String> for BoxedString {
    fn into(self) -> String {
        #[allow(unsafe_code)]
        let s = unsafe { String::from_raw_parts(self.ptr.as_ptr(), self.len(), self.capacity()) };
        forget(self);
        s
    }
}
