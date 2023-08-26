// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use core::ptr::NonNull;
use core::num::NonZeroUsize;
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenericTaggedPtr<T, const MASK: usize, const PATTERN: usize>(NonZeroUsize, PhantomData<T>);
impl<T, const MASK: usize, const PATTERN: usize> GenericTaggedPtr<T, MASK, PATTERN> {
    pub(crate) fn new(v: *mut T) -> Option<Self> {
        // this will be optimized away at compile time
        if PATTERN == 0 || PATTERN & MASK != PATTERN {
            panic!("fill must not be zero");
        }

        if v as usize == 0 || v as usize & MASK != 0 {
            None
        } else {
            #[allow(unsafe_code)]
            let ptr = unsafe { NonZeroUsize::new_unchecked((v as usize & !MASK) | PATTERN) }; // SAFETY: v | PATTERN != 0 because PATTERN != 0
            Some(Self(ptr, PhantomData))
        }
    }
    pub(crate) fn as_non_null(self) -> NonNull<T> {
        #[allow(unsafe_code)]
        unsafe { NonNull::new_unchecked((self.0.get() & !MASK) as *mut T) } // SAFETY: v & !MASK != 0 guaranteed from Self::new
    }
}

pub(crate) type TaggedPtr = GenericTaggedPtr<u8, 3, 2>;

#[test]
fn check_filled_ptr() {
    for i in 1..=1024 {
        let v = i << 2;
        let p = TaggedPtr::new(v as *mut _).unwrap();
        assert_eq!(p.0.get(), v | 2);
        assert_eq!(p.as_non_null().as_ptr() as usize, v);
    }
}