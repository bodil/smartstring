// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{boxed::{BoxedString, PseudoString}, inline::InlineString, SmartString};
use static_assertions::{assert_eq_size, const_assert, const_assert_eq};
use std::mem::{align_of, size_of, MaybeUninit};

/// A compact string representation equal to [`String`][String] in size with guaranteed inlining.
///
/// This representation relies on pointer alignment to be able to store a discriminant bit in its
/// inline form that will never be present in its [`String`][String] form, thus
/// giving us 24 bytes on 64-bit architectures, and 12 bytes on 32-bit, minus one bit, to encode our
/// inline string. It uses the rest of the discriminant bit's byte to encode the string length, and
/// the remaining bytes (23 or 11 depending on arch) to store the string data. When the available space is exceeded,
/// it swaps itself out with a [`String`][String] containing its previous
/// contents, relying on the discriminant bit in the [`String`][String]'s pointer to be unset, so we can
/// store the [`String`][String] safely without taking up any extra space for a discriminant.
///
/// This performs generally as well as [`String`][String] on all ops on boxed strings, and
/// better than [`String`][String]s on inlined strings.
///
/// [String]: https://doc.rust-lang.org/std/string/struct.String.html
#[derive(Debug)]
pub struct Compact;

/// A representation similar to [`Compact`][Compact] but which doesn't re-inline strings.
///
/// This is a variant of [`Compact`][Compact] which doesn't aggressively inline strings.
/// Where [`Compact`][Compact] automatically turns a heap allocated string back into an
/// inlined string if it should become short enough, [`LazyCompact`][LazyCompact] keeps
/// it heap allocated once heap allocation has occurred. If your aim is to defer heap
/// allocation as much as possible, rather than to ensure cache locality, this is the
/// variant you want - it won't allocate until the inline capacity is exceeded, and it
/// also won't deallocate once allocation has occurred, which risks reallocation if the
/// string exceeds its inline capacity in the future.
///
/// [Compact]: struct.Compact.html
/// [String]: https://doc.rust-lang.org/std/string/struct.String.html
#[derive(Debug)]
pub struct LazyCompact;

/// Marker trait for [`SmartString`][SmartString] representations.
///
/// See [`LazyCompact`][LazyCompact] and [`Compact`][Compact].
///
/// [SmartString]: struct.SmartString.html
/// [Compact]: struct.Compact.html
/// [LazyCompact]: struct.LazyCompact.html
///
/// Implementing this trait is extremely unsafe and not recommended
/// The requirements are that:
/// * std::mem::size_of<DiscriminantContainer> == std::mem::size_of<usize>
/// * std::mem::align_of<DiscriminantContainer> == std::mem::align_of<usize>
/// * std::mem::size_of<BoxedString> == std::size_of<String>
/// * std::mem::align_of<BoxedString> == std::mem::align_of<String>
/// * It should be always safe to transmute from BoxedString into SmartString<Mode>
/// * The highmost bit of BoxedString must be one
/// * If the highest std::mem::size_of<usize> bytes of BoxedString were casted into DiscriminantContainer
/// at any time, even in methods of BoxedString, it must be a valid DiscriminantContainer.
pub unsafe trait SmartStringMode {
    /// The boxed string type for this layout.
    type BoxedString: BoxedString;
    /// The maximum capacity of an inline string, in bytes.
    const MAX_INLINE: usize;
    /// A constant to decide whether to turn a wrapped string back into an inlined
    /// string whenever possible (`true`) or leave it as a wrapped string once wrapping
    /// has occurred (`false`).
    const DEALLOC: bool;
    /// Unfortunately const generics don't exists at the time of writing
    /// If DEALLOC == true or cfg!(feature = "lazy_null_pointer_optimizations") == true, this should be std::num::NonZeroUsize,
    /// Otherwise it should be PossiblyZeroSize
    type DiscriminantContainer: DiscriminantContainer;
}

/// Contains the discriminant. This is a visible field in the SmartString struct, so the compiler
/// is able to make null pointer optimizations when the type allows them.
pub trait DiscriminantContainer {
    /// Returns the full marker byte
    fn get_full_marker(&self) -> u8;
    /// Return Self with the requirement that the marker is inside
    fn new(marker: u8) -> Self;
    /// Flip the highest bit of marker
    ///
    /// # Safety
    ///
    /// Caller must ensure this doesn't cause UB, for example by turning a Non-zero DiscriminantContainer into a zeroed one
    unsafe fn flip_bit(&mut self);
}

impl DiscriminantContainer for std::num::NonZeroUsize {
    fn get_full_marker(&self) -> u8 {
        (self.get() >> ((std::mem::size_of::<usize>() - 1)*8)) as u8
    }
    fn new(marker: u8) -> Self {
        unsafe {
            Self::new_unchecked(
                ((marker as usize) << ((std::mem::size_of::<usize>() - 1)*8)) + 1
            )
        }
    }
    unsafe fn flip_bit(&mut self) {
        *self = Self::new_unchecked(self.get());
    }
}

/// A structure that stores a marker and raw data
#[cfg(target_endian = "big")]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
struct PossiblyZeroSize {
    marker: u8,
    data: [MaybeUninit<u8>; std::mem::size_of::<usize>() - 1],
}

/// A structure that stores a marker and raw data
#[cfg(target_endian = "little")]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[derive(Debug)]
pub struct PossiblyZeroSize {
    data: [MaybeUninit<u8>; std::mem::size_of::<usize>() - 1],
    marker: u8,
}

impl DiscriminantContainer for PossiblyZeroSize {
    fn get_full_marker(&self) -> u8 {
        self.marker
    }
    fn new(marker: u8) -> Self {
        Self {
            marker,
            data: [MaybeUninit::uninit(); std::mem::size_of::<usize>() - 1],
        }
    }
    unsafe fn flip_bit(&mut self) {
        self.marker^= 128;
    }
}

unsafe impl SmartStringMode for Compact {
    type BoxedString = PseudoString;
    const MAX_INLINE: usize = size_of::<String>() - 1;
    const DEALLOC: bool = true;
    type DiscriminantContainer = std::num::NonZeroUsize;
}


#[cfg(not(feature = "lazy_null_pointer_optimizations"))]
unsafe impl SmartStringMode for LazyCompact {
    type BoxedString = PseudoString;
    const MAX_INLINE: usize = size_of::<String>() - 1;
    const DEALLOC: bool = false;
    type DiscriminantContainer = PossiblyZeroSize;
}

#[cfg(feature = "lazy_null_pointer_optimizations")]
unsafe impl SmartStringMode for LazyCompact {
    type BoxedString = PseudoString;
    const MAX_INLINE: usize = size_of::<String>() - 1;
    const DEALLOC: bool = false;
    type DiscriminantContainer = std::num::NonZeroUsize;
}

// Assert that we're not using more space than we can encode in the header byte,
// just in case we're on a 1024-bit architecture.
const_assert!(<Compact as SmartStringMode>::MAX_INLINE < 128);
const_assert!(<LazyCompact as SmartStringMode>::MAX_INLINE < 128);

// Assert that all the structs are of the expected size.
assert_eq_size!(
    <Compact as SmartStringMode>::BoxedString,
    SmartString<Compact>
);
assert_eq_size!(
    <LazyCompact as SmartStringMode>::BoxedString,
    SmartString<LazyCompact>
);
assert_eq_size!(InlineString<Compact>, SmartString<Compact>);
assert_eq_size!(InlineString<LazyCompact>, SmartString<LazyCompact>);

assert_eq_size!(String, SmartString<Compact>);
assert_eq_size!(String, SmartString<LazyCompact>);

assert_eq_size!(SmartString<Compact>, Option<SmartString<Compact>>);
#[cfg(feature = "lazy_null_pointer_optimizations")]
assert_eq_size!(SmartString<LazyCompact>, Option<SmartString<LazyCompact>>);

// Assert that `SmartString` is aligned correctly.
const_assert_eq!(align_of::<String>(), align_of::<SmartString<Compact>>());
const_assert_eq!(align_of::<String>(), align_of::<SmartString<LazyCompact>>());
