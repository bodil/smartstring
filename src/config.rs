// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    boxed::{BoxedString, FragmentString},
    inline::InlineString,
    SmartString,
};
use static_assertions::{assert_eq_size, const_assert, const_assert_eq};
use std::mem::{align_of, size_of};

/// The capacity of the prefix fragment stored by [`Prefixed`][Prefixed] [`SmartString`][SmartString]s.
///
/// [SmartString]: struct.SmartString.html
/// [Prefixed]: struct.Prefixed.html
pub const FRAGMENT_SIZE: usize = align_of::<String>() - 1;

/// A compact string representation equal to [`String`][String] in size.
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

/// A string representation that always keeps an inline prefix.
///
/// This layout is optimised for use cases with frequent comparisons of long strings
/// using [`Ord`][Ord] or [`Eq`][Eq].
///
/// This looks similar to [`Compact`][Compact] when inlined, except it has one pointer's
/// length's worth of extra space - 31 bytes in total on 64-bit architectures, 15 on 32-bit.
/// The boxed variant copies up to [`FRAGMENT_SIZE`][FRAGMENT_SIZE] bytes of the string
/// into its local representation, in order to avoid dereferencing as much as possible
/// when checking for equality and ordering, and uses the same byte as its inline version
/// to store the discriminant bit plus the length of the fragment (which may be below
/// [`FRAGMENT_SIZE`][FRAGMENT_SIZE] if it contains multibyte UTF-8 characters).
///
/// ## When To Use This?
///
/// This comes with an overhead on many operations, and on top of that, the [`Ord`][Ord]
/// implementation is necessarily more complex because it's dealing with multiple UTF-8
/// slices, and comparing them isn't nontrivial. Especially, when comparing the prefix
/// against something with an unknown amount of `char`s, or where you don't know exactly
/// where the relevant `char` boundary is up front, we have to track the `char` count as
/// we compare, which is considerably slower than a simple `memcmp`. For this reason,
/// always compare [`SmartString`][SmartString]s to other [`SmartString`][SmartString]s,
/// not to string slices, because they track their prefix `char` counts and can optimise
/// for when they're identical.
///
/// This performs best when the prefix is likely to be able to quickly decide
/// non-equivalence, and when the full comparison is likely to have to do enough work
/// to compensate for the extra overhead of the prefix check. In other words, it
/// outperforms regular strings and [`Compact`][Compact] layout [`SmartString`][SmartString]s
/// when you have long keys that rarely share a common prefix. If your keys tend to be
/// short, or if common prefixes are the norm, you're much better off using
/// [`Compact`][Compact]. As a rule, when in doubt: benchmark.
///
/// [SmartString]: struct.SmartString.html
/// [Compact]: struct.Compact.html
/// [FRAGMENT_SIZE]: constant.FRAGMENT_SIZE.html
/// [String]: https://doc.rust-lang.org/std/string/struct.String.html
#[derive(Debug)]
pub struct Prefixed;

/// Marker trait for [`SmartString`][SmartString] representations.
///
/// See [`Compact`][Compact] and [`Prefixed`][Prefixed].
///
/// [SmartString]: struct.SmartString.html
/// [Compact]: struct.Compact.html
/// [Prefixed]: struct.Prefixed.html
pub trait SmartStringMode {
    /// The boxed string type for this layout.
    type BoxedString: BoxedString + From<String>;
    /// The inline string type for this layout.
    type InlineArray: AsRef<[u8]> + AsMut<[u8]> + Clone + Copy;
    /// The maximum capacity of an inline string, in bytes.
    const MAX_INLINE: usize;
    /// A constant to decide whether to use [`Prefixed`][Prefixed] optimisations
    /// when comparing, in the absence of specialisation.
    ///
    /// [Prefixed]: struct.Prefixed.html
    const PREFIXED: bool;
}

impl SmartStringMode for Compact {
    type BoxedString = String;
    type InlineArray = [u8; size_of::<String>() - 1];
    const MAX_INLINE: usize = size_of::<String>() - 1;
    const PREFIXED: bool = false;
}

impl SmartStringMode for Prefixed {
    type BoxedString = FragmentString;
    type InlineArray = [u8; size_of::<FragmentString>() - 1];
    const MAX_INLINE: usize = size_of::<FragmentString>() - 1;
    const PREFIXED: bool = true;
}

// Assert that we're not using more space than we can encode in the header byte,
// just in case we're on a 1024-bit architecture.
const_assert!(<Compact as SmartStringMode>::MAX_INLINE < 128);
const_assert!(<Prefixed as SmartStringMode>::MAX_INLINE < 128);

// Assert that all the structs are of the expected size.
assert_eq_size!(
    <Compact as SmartStringMode>::BoxedString,
    SmartString<Compact>
);
assert_eq_size!(
    <Prefixed as SmartStringMode>::BoxedString,
    SmartString<Prefixed>
);
assert_eq_size!(InlineString<Compact>, SmartString<Compact>);
assert_eq_size!(InlineString<Prefixed>, SmartString<Prefixed>);

// Assert that `SmartString` is aligned correctly.
const_assert_eq!(align_of::<String>(), align_of::<SmartString<Compact>>());
const_assert_eq!(align_of::<String>(), align_of::<SmartString<Prefixed>>());
