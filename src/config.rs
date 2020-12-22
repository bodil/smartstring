// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{boxed::BoxedString, inline::InlineString, SmartString};
use alloc::string::String;
use core::mem::{align_of, size_of};
use static_assertions::{assert_cfg, assert_eq_size, const_assert, const_assert_eq};

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
pub trait SmartStringMode {
    /// The boxed string type for this layout.
    type BoxedString: BoxedString + From<String>;
    /// The inline string type for this layout.
    type InlineArray: AsRef<[u8]> + AsMut<[u8]> + Clone + Copy;
    /// The maximum capacity of an inline string, in bytes.
    const MAX_INLINE: usize;
    /// A constant to decide whether to turn a wrapped string back into an inlined
    /// string whenever possible (`true`) or leave it as a wrapped string once wrapping
    /// has occurred (`false`).
    const DEALLOC: bool;
}

impl SmartStringMode for Compact {
    type BoxedString = String;
    type InlineArray = [u8; size_of::<String>() - 1];
    const MAX_INLINE: usize = size_of::<String>() - 1;
    const DEALLOC: bool = true;
}

impl SmartStringMode for LazyCompact {
    type BoxedString = String;
    type InlineArray = [u8; size_of::<String>() - 1];
    const MAX_INLINE: usize = size_of::<String>() - 1;
    const DEALLOC: bool = false;
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

// Assert that `SmartString` is aligned correctly.
const_assert_eq!(align_of::<String>(), align_of::<SmartString<Compact>>());
const_assert_eq!(align_of::<String>(), align_of::<SmartString<LazyCompact>>());

// This hack isn't going to work out very well on 32-bit big endian archs,
// so let's not compile on those.
assert_cfg!(not(all(target_endian = "big", target_pointer_width = "32")));
