// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! # Smart String
//!
//! [`SmartString`][SmartString] is a wrapper around [`String`][String] which offers
//! automatic inlining of small strings. It comes in two flavours:
//! [`LazyCompact`][LazyCompact], which takes up exactly as much space as a [`String`][String]
//! and is generally a little faster, and [`Compact`][Compact], which is the same as
//! [`LazyCompact`][LazyCompact] except it will aggressively re-inline any expanded
//! [`String`][String]s which become short enough to do so.
//! [`LazyCompact`][LazyCompact] is the default.
//!
//! ## What Is It For?
//!
//! The intended use for [`SmartString`][SmartString] is as a key type for a
//! B-tree (such as [`std::collections::BTreeMap`][BTreeMap]) or any kind of
//! array operation where cache locality is critical.
//!
//! In general, it's a nice data type for reducing your heap allocations and
//! increasing the locality of string data. If you use [`SmartString`][SmartString]
//! as a drop-in replacement for [`String`][String], you're almost certain to see
//! a slight performance boost, as well as slightly reduced memory usage.
//!
//! How To Use It?
//!
//! [`SmartString`][SmartString] has the exact same API as [`String`][String],
//! all the clever bits happen automatically behind the scenes, so you could just:
//!
//! ```rust
//! use smartstring::alias::String;
//! use std::fmt::Write;
//!
//! let mut string = String::new();
//! string.push_str("This is just a string!");
//! string.clear();
//! write!(string, "Hello Joe!");
//! assert_eq!("Hello Joe!", string);
//! ```
//!
//! ## Give Me The Details
//!
//! [`SmartString`][SmartString] is the same size as [`String`][String] and
//! relies on restrictions on allocation size to be able to store a discriminant bit in its
//! inline form. More specifically the length and capacity of a string must fit in a isize,
//! which means that the most significant bit of those fields will always be zero. This
//! gives us 24 bytes (on 64-bit architectures) minus one bit to encode our
//! inline string. It uses 23 bytes to store the string data and the remaining
//! 7 bits to encode the string's length. When the available space is exceeded,
//! it swaps itself out with a [`String`][String] containing its previous
//! contents. Likewise, if the string's length should drop below its inline
//! capacity again, it deallocates the string and moves its contents inline.
//!
//! It is aggressive about inlining strings, meaning that if you modify a heap allocated
//! string such that it becomes short enough for inlining, it will be inlined immediately
//! and the allocated [`String`][String] will be dropped. This may cause multiple
//! unintended allocations if you repeatedly adjust your string's length across the
//! inline capacity threshold, so if your string's construction can get
//! complicated and you're relying on performance during construction, it might be better
//! to construct it as a [`String`][String] and convert it once construction is done.
//!
//! [`LazyCompact`][LazyCompact] looks the same as [`Compact`][Compact], except
//! it never re-inlines a string that's already been heap allocated, instead
//! keeping the allocation around in case it needs it. This makes for less
//! cache local strings, but is the best choice if you're more worried about
//! time spent on unnecessary allocations than cache locality. By default
//! LazyCompact
//!
//! ## Performance
//!
//! It doesn't aim to be more performant than [`String`][String] in the general case,
//! except that it doesn't trigger heap allocations for anything shorter than
//! its inline capacity and so can be reasonably expected to exceed
//! [`String`][String]'s performance perceptibly on shorter strings, as well as being more
//! memory efficient in these cases. There will always be a slight overhead on all
//! operations on boxed strings, compared to [`String`][String].
//!
//! ## Null pointer optimizations
//!
//! By default, null pointer optimizations are enabled for SmartString,
//! so size_of<SmartString> == size_of<Option<SmartString>>.
//! It's possible to disable them for SmartString<LazyCompact>, for a very small performance gain,
//! as it usually means that the bytes of SmartString<LazyCompact> can be reinterpreted as a String.
//! To do this, disable the feature flag lazy_null_pointer_optimizations
//!
//! ## Feature Flags
//!
//! `smartstring` comes with optional support for the following crates through Cargo
//! feature flags. You can enable them in your `Cargo.toml` file like this:
//!
//! ```no_compile
//! [dependencies]
//! smartstring = { version = "*", features = ["proptest", "serde"] }
//! ```
//!
//! | Feature | Description |
//! | ------- | ----------- |
//! | [`arbitrary`](https://crates.io/crates/arbitrary) | [`Arbitrary`][Arbitrary] implementation for [`SmartString`][SmartString]. |
//! | [`proptest`](https://crates.io/crates/proptest) | A strategy for generating [`SmartString`][SmartString]s from a regular expression. |
//! | [`serde`](https://crates.io/crates/serde) | [`Serialize`][Serialize] and [`Deserialize`][Deserialize] implementations for [`SmartString`][SmartString]. |
//!
//! [SmartString]: struct.SmartString.html
//! [LazyCompact]: struct.LazyCompact.html
//! [Compact]: struct.Compact.html
//! [IntoString]: struct.SmartString.html#impl-Into%3CString%3E
//! [String]: https://doc.rust-lang.org/std/string/struct.String.html
//! [BTreeMap]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
//! [eq]: https://doc.rust-lang.org/std/cmp/trait.PartialEq.html#tymethod.eq
//! [cmp]: https://doc.rust-lang.org/std/cmp/trait.Ord.html#tymethod.cmp
//! [transmute]: https://doc.rust-lang.org/std/mem/fn.transmute.html
//! [tinystr]: https://crates.io/crates/tinystr
//! [serde]: https://crates.io/crates/serde
//! [Serialize]: https://docs.rs/serde/latest/serde/trait.Serialize.html
//! [Deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
//! [Arbitrary]: https://docs.rs/arbitrary/latest/arbitrary/trait.Arbitrary.html

#![forbid(rust_2018_idioms)]
#![deny(nonstandard_style)]
#![warn(unreachable_pub, missing_debug_implementations, missing_docs)]

use std::{
    borrow::{Borrow, BorrowMut},
    cmp::Ordering,
    convert::Infallible,
    fmt::{Debug, Display, Error, Formatter, Write},
    hash::{Hash, Hasher},
    iter::FromIterator,
    mem::MaybeUninit,
    ops::{
        Add, Bound, Deref, DerefMut, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull,
        RangeInclusive, RangeTo, RangeToInclusive,
    },
    str::FromStr,
};

mod config;
pub use config::{Compact, LazyCompact, SmartStringMode, DiscriminantContainer, PossiblyZeroSize};

mod marker_byte;
use marker_byte::Discriminant;

mod inline;
use inline::InlineString;

mod boxed;
use boxed::BoxedString;

mod casts;
use casts::{StringCast, StringCastInto, StringCastMut, please_transmute};

mod iter;
pub use iter::Drain;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "arbitrary")]
mod arbitrary;

#[cfg(feature = "proptest")]
pub mod proptest;

/// Convenient type aliases.
pub mod alias {
    use super::*;

    /// A convenience alias for a [`LazyCompact`][LazyCompact] layout [`SmartString`][SmartString].
    ///
    /// Just pretend it's a [`String`][String]!
    ///
    /// [SmartString]: struct.SmartString.html
    /// [LazyCompact]: struct.LazyCompact.html
    pub type String = SmartString<LazyCompact>;

    /// A convenience alias for a [`Compact`][Compact] layout [`SmartString`][SmartString].
    ///
    /// [SmartString]: struct.SmartString.html
    /// [Compact]: struct.Compact.html
    pub type CompactString = SmartString<Compact>;
}

/// A smart string.
///
/// This wraps one of two string types: an inline string or a boxed string.
/// Conversion between the two happens opportunistically and transparently.
///
/// It takes a layout as its type argument: one of [`Compact`][Compact] or [`LazyCompact`][LazyCompact].
///
/// It mimics the interface of [`String`][String] except where behaviour cannot
/// be guaranteed to stay consistent between its boxed and inline states. This means
/// you still have `capacity()` and `shrink_to_fit()`, relating to state that only
/// really exists in the boxed variant, because the inline variant can still give
/// sensible behaviour for these operations, but `with_capacity()`, `reserve()` etc are
/// absent, because they would have no effect on inline strings and the requested
/// state changes wouldn't carry over if the inline string is promoted to a boxed
/// one - not without also storing that state in the inline representation, which
/// would waste precious bytes for inline string data.
///
/// [SmartString]: struct.SmartString.html
/// [Compact]: struct.Compact.html
/// [LazyCompact]: struct.LazyCompact.html
/// [String]: https://doc.rust-lang.org/std/string/struct.String.html

#[cfg(target_endian = "big")]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
pub struct SmartString<Mode: SmartStringMode> {
    disc: Mode::DiscriminantContainer,
    data: [MaybeUninit<u8>; 2*std::mem::size_of::<usize>()],
}

//Documentation needs to be duplicated

/// A smart string.
///
/// This wraps one of two string types: an inline string or a boxed string.
/// Conversion between the two happens opportunistically and transparently.
///
/// It takes a layout as its type argument: one of [`Compact`][Compact] or [`LazyCompact`][LazyCompact].
///
/// It mimics the interface of [`String`][String] except where behaviour cannot
/// be guaranteed to stay consistent between its boxed and inline states. This means
/// you still have `capacity()` and `shrink_to_fit()`, relating to state that only
/// really exists in the boxed variant, because the inline variant can still give
/// sensible behaviour for these operations, but `with_capacity()`, `reserve()` etc are
/// absent, because they would have no effect on inline strings and the requested
/// state changes wouldn't carry over if the inline string is promoted to a boxed
/// one - not without also storing that state in the inline representation, which
/// would waste precious bytes for inline string data.
///
/// [SmartString]: struct.SmartString.html
/// [Compact]: struct.Compact.html
/// [LazyCompact]: struct.LazyCompact.html
/// [String]: https://doc.rust-lang.org/std/string/struct.String.html

#[cfg(target_endian = "little")]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
pub struct SmartString<Mode: SmartStringMode> {
    data: [MaybeUninit<u8>; 2*std::mem::size_of::<usize>()],
    disc: Mode::DiscriminantContainer,
}

impl<Mode: SmartStringMode> Drop for SmartString<Mode> {
    fn drop(&mut self) {
        if let StringCastMut::Boxed(mut string) = self.cast_mut() {
            string.clear();
        }
    }
}

impl<Mode: SmartStringMode> Clone for SmartString<Mode> {
    /// Clone a `SmartString`.
    ///
    /// If the string is small enough to fit inline, this is a [`Copy`][Copy] operation. Otherwise,
    /// [`String::clone()`][String::clone] is invoked.
    ///
    /// [String::clone]: https://doc.rust-lang.org/std/string/struct.String.html#impl-Clone
    /// [Copy]: https://doc.rust-lang.org/std/marker/trait.Copy.html
    fn clone(&self) -> Self {
        match self.cast() {
            StringCast::Boxed(_) => Self::from(self.deref()),
            StringCast::Inline(string) => Self::from_inline(*string),
        }
    }
}

impl<Mode: SmartStringMode> Deref for SmartString<Mode> {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        match self.cast() {
            StringCast::Boxed(string) => string,
            StringCast::Inline(string) => string,
        }
    }
}

impl<Mode: SmartStringMode> DerefMut for SmartString<Mode> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { match self.discriminant() {
            Discriminant::Boxed => {
                let boxed : &mut Mode::BoxedString = &mut *(self as *mut Self).cast();
                boxed
            }
            Discriminant::Inline => {
                let inline : &mut InlineString<Mode> = &mut *(self as *mut Self).cast();
                inline
            }
        }}
    }
}

impl<Mode: SmartStringMode> SmartString<Mode> {
    /// Construct an empty string.
    #[inline(always)]
    pub fn new() -> Self {
        Self::from_inline(InlineString::new())
    }

    fn from_boxed(boxed: Mode::BoxedString) -> Self {
        unsafe {
            please_transmute(boxed)
        }
    }

    fn from_inline(inline: InlineString<Mode>) -> Self {
        unsafe {
            please_transmute(inline)
        }
    }

    fn discriminant(&self) -> Discriminant {
        if self.disc.get_full_marker() & 128 != 0 {
            Discriminant::Inline
        } else {
            Discriminant::Boxed
        }
    }

    fn cast(&self) -> StringCast<'_, Mode> {
        match self.discriminant() {
            Discriminant::Inline => StringCast::Inline(unsafe { &*(self as *const Self).cast() }),
            Discriminant::Boxed => StringCast::Boxed(unsafe { &*(self as *const Self).cast() }),
        }
    }

    fn cast_mut(&mut self) -> StringCastMut<'_, Mode> {
        match self.discriminant() {
            Discriminant::Inline => StringCastMut::Inline(unsafe { &mut *(self as *mut Self).cast() }),
            Discriminant::Boxed => {
                StringCastMut::Boxed( unsafe {
                    boxed::StringReference::from_smart_unchecked(self)
                })
            }
        }
    }

    fn cast_into(self) -> StringCastInto<Mode> {
        match self.discriminant() {
            Discriminant::Inline => StringCastInto::Inline(unsafe { please_transmute(self) }),
            Discriminant::Boxed => StringCastInto::Boxed(unsafe { please_transmute(self) }),
        }
    }

    //Unsafe: if nullptr optimizations are on, string must have nonzero size or capacity
    //depending on the laziness
    unsafe fn promote_from(&mut self, string: String) {
        debug_assert!(self.discriminant() == Discriminant::Inline);
        let ptr = (self as *mut Self).cast();
        //Use ptr::write to avoid redundant dropping
        std::ptr::write(ptr, boxed::PseudoString::from_string_unchecked(string));
    }

    /// Attempt to inline the string regardless of whether `Mode::DEALLOC` is set.
    fn really_try_demote(&mut self) -> bool {
        let inlined = if let StringCastMut::Boxed(string) = self.cast_mut() {
            if string.len() > Mode::MAX_INLINE {
                return false;
            } else {
                let deref: &str = string.deref();
                Self::from(deref)
            }
        } else {
            return true;
        };
        std::mem::forget(
            std::mem::replace(self, inlined)
        );
        true
    }

    /// Return the length in bytes of the string.
    ///
    /// Note that this may differ from the length in `char`s.
    pub fn len(&self) -> usize {
        match self.cast() {
            StringCast::Boxed(string) => string.len(),
            StringCast::Inline(string) => string.len(),
        }
    }

    /// Test whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Test whether the string is currently inlined.
    pub fn is_inline(&self) -> bool {
        self.discriminant() == Discriminant::Inline
    }

    /// Get a reference to the string as a string slice.
    pub fn as_str(&self) -> &str {
        self.deref()
    }

    /// Get a reference to the string as a mutable string slice.
    pub fn as_mut_str(&mut self) -> &mut str {
        self.deref_mut()
    }

    /// Push a character to the end of the string.
    pub fn push(&mut self, ch: char) {
        let promote = match self.cast_mut() {
            StringCastMut::Boxed(mut string) => {
                string.push(ch);
                return;
            }
            StringCastMut::Inline(string) => {
                let len = string.len();
                let new_len = len + ch.len_utf8();
                if new_len > Mode::MAX_INLINE {
                    let mut new_str = String::with_capacity(new_len);
                    new_str.push_str(string);
                    new_str.push(ch);
                    new_str
                } else {
                    for e in &mut string.data[len..new_len] {
                        //These have to be initialized, as we are passing u8:s into encode_utf8
                        *e = std::mem::MaybeUninit::new(0);
                    }
                    let written = ch.encode_utf8( unsafe {
                        &mut *(&mut string.data[len..new_len] as *mut [std::mem::MaybeUninit<u8>] as *mut [u8])
                    }).len();
                    unsafe { string.set_len(len + written) };
                    return;
                }
            }
        };
        unsafe {self.promote_from(promote)};
    }

    /// Copy a string slice onto the end of the string.
    pub fn push_str(&mut self, string: &str) {
        let len = self.len();
        let promote = match self.cast_mut() {
            StringCastMut::Boxed(mut this) => {
                this.push_str(string);
                return;
            }
            StringCastMut::Inline(this) => {
                let new_len = len + string.len();
                if new_len > Mode::MAX_INLINE {
                    let mut new_str = String::with_capacity(new_len);
                    new_str.push_str(this);
                    new_str.push_str(string);
                    new_str
                } else {
                    unsafe {
                        this.as_mut_slice()[len..new_len].copy_from_slice(
                            &*(string.as_bytes() as *const [u8] as *const [std::mem::MaybeUninit<u8>])
                        );
                        this.set_len(new_len);
                    }
                    return
                }
            }
        };
        unsafe {self.promote_from(promote)};
    }

    /// Return the currently allocated capacity of the string.
    ///
    /// Note that if this is a boxed string, it returns [`String::capacity()`][String::capacity],
    /// but an inline string always returns [`SmartStringMode::MAX_INLINE`][MAX_INLINE].
    ///
    /// Note also that if a boxed string is converted into an inline string, its capacity is
    /// deallocated, and if the inline string is promoted to a boxed string in the future,
    /// it will be reallocated with a default capacity.
    ///
    /// [MAX_INLINE]: trait.SmartStringMode.html#associatedconstant.MAX_INLINE
    /// [String::capacity]: https://doc.rust-lang.org/std/string/struct.String.html#method.capacity
    pub fn capacity(&self) -> usize {
        if let StringCast::Boxed(string) = self.cast() {
            string.capacity()
        } else {
            Mode::MAX_INLINE
        }
    }

    /// Shrink the capacity of the string to fit its contents exactly.
    ///
    /// This has no effect on inline strings, which always have a fixed capacity.
    /// Thus, it's not safe to assume that [`capacity()`][capacity] will
    /// equal [`len()`][len] after calling this.
    ///
    /// Calling this on a [`LazyCompact`][LazyCompact] string that is currently
    /// heap allocated but is short enough to be inlined will deallocate the
    /// heap allocation and convert it to an inline string.
    ///
    /// [capacity]: struct.SmartString.html#method.capacity
    /// [len]: struct.SmartString.html#method.len
    pub fn shrink_to_fit(&mut self) {
        if let StringCastMut::Boxed(mut string) = self.cast_mut() {
            if string.len() > Mode::MAX_INLINE {
                string.shrink_to_fit();
                return;
            }
        }
        if !Mode::DEALLOC && !cfg!(lazy_null_pointer_optimizations) {
            self.really_try_demote();
        }
        //Else: the demotion is already done by StringReference
    }

    /// Truncate the string to `new_len` bytes.
    ///
    /// If `new_len` is larger than the string's current length, this does nothing.
    /// If `new_len` isn't on a UTF-8 character boundary, this method panics.
    pub fn truncate(&mut self, new_len: usize) {
        match self.cast_mut() {
            StringCastMut::Boxed(mut string) => string.truncate(new_len),
            StringCastMut::Inline(string) => {
                if new_len < string.len() {
                    assert!(string.is_char_boundary(new_len));
                    unsafe {
                        string.set_len(new_len)
                    };
                }
            }
        }
    }

    /// Pop a `char` off the end of the string.
    pub fn pop(&mut self) -> Option<char> {
        let result = match self.cast_mut() {
            StringCastMut::Boxed(mut string) => string.pop()?,
            StringCastMut::Inline(string) => {
                let ch = string.chars().rev().next()?;
                unsafe {string.set_len(string.len() - ch.len_utf8());}
                return Some(ch);
            }
        };
        Some(result)
    }

    /// Remove a `char` from the string at the given index.
    ///
    /// If the index doesn't fall on a UTF-8 character boundary, this method panics.
    pub fn remove(&mut self, index: usize) -> char {
        match self.cast_mut() {
            StringCastMut::Boxed(mut string) => string.remove(index),
            StringCastMut::Inline(string) => {
                let ch = match string[index..].chars().next() {
                    Some(ch) => ch,
                    None => panic!("cannot remove a char from the end of a string"),
                };
                let next = index + ch.len_utf8();
                let len = string.len();
                let tail_len = len - next;
                unsafe {
                    if tail_len > 0 {
                        string.data[index].as_mut_ptr().copy_from(string.data[next].as_ptr(), len - next);
                    }
                    string.set_len(len - (next - index));
                }
                ch
            }
        }
    }

    /// Insert a `char` into the string at the given index.
    ///
    /// If the index doesn't fall on a UTF-8 character boundary, this method panics.
    pub fn insert(&mut self, index: usize, ch: char) {
        let promote = match self.cast_mut() {
            StringCastMut::Boxed(mut string) => {
                string.insert(index, ch);
                return;
            }
            StringCastMut::Inline(string) if string.len() + ch.len_utf8() <= Mode::MAX_INLINE => {
                if !string.is_char_boundary(index) {
                    panic!();
                }
                let mut buffer = [0; 4];
                let buffer = ch.encode_utf8(&mut buffer).as_bytes();
                unsafe {
                    string.insert_bytes(index, buffer);
                }
                return;
            }
            StringCastMut::Inline(string) => {
                let mut string = string.to_string();
                string.insert(index, ch);
                string
            }
        };
        unsafe {self.promote_from(promote)};
    }

    /// Insert a string slice into the string at the given index.
    ///
    /// If the index doesn't fall on a UTF-8 character boundary, this method panics.
    pub fn insert_str(&mut self, index: usize, string: &str) {
        let promote = match self.cast_mut() {
            StringCastMut::Boxed(mut this) => {
                this.insert_str(index, string);
                return;
            }
            StringCastMut::Inline(this) if this.len() + string.len() <= Mode::MAX_INLINE => {
                if !this.is_char_boundary(index) {
                    panic!();
                }
                unsafe {this.insert_bytes(index, string.as_bytes())};
                return;
            },
            StringCastMut::Inline(this) => {
                let mut this = this.to_string();
                this.insert_str(index, string);
                this
            }
        };
        unsafe {self.promote_from(promote)};
    }

    /// Split the string into two at the given index.
    ///
    /// Returns the content to the right of the index as a new string, and removes
    /// it from the original.
    ///
    /// If the index doesn't fall on a UTF-8 character boundary, this method panics.
    pub fn split_off(&mut self, index: usize) -> Self {
        let result = match self.cast_mut() {
            StringCastMut::Boxed(mut string) => string.split_off(index),
            StringCastMut::Inline(string) => {
                assert!(string.is_char_boundary(index));
                let result = string[index..].into();
                unsafe {
                    string.set_len(index);
                }
                return result;
            }
        };
        result.into()
    }

    /// Clear the string.
    ///
    /// This causes any memory reserved by the string to be immediately deallocated.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Filter out `char`s not matching a predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(char) -> bool,
    {
        match self.cast_mut() {
            StringCastMut::Boxed(mut string) => string.retain(f),
            StringCastMut::Inline(string) => {
                let len = string.len();
                let mut del_bytes = 0;
                let mut index = 0;

                while index < len {
                    let ch = unsafe {
                        string
                            .get_unchecked(index..len)
                            .chars()
                            .next()
                            .unwrap()
                    };
                    let ch_len = ch.len_utf8();

                    if !f(ch) {
                        del_bytes += ch_len;
                    } else if del_bytes > 0 {
                        unsafe {
                            let ptr = string.as_mut_slice().as_mut_ptr();
                            ptr.add(index - del_bytes).copy_from(ptr.add(index), ch_len);
                        };
                    }
                    index += ch_len;
                }
                if del_bytes > 0 {
                    unsafe {
                        string.set_len(len - del_bytes);
                    }
                }
            }
        }
    }

    /// Construct a draining iterator over a given range.
    ///
    /// This removes the given range from the string, and returns an iterator over the
    /// removed `char`s.
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, Mode>
    where
        R: RangeBounds<usize>,
    {
        Drain::new(self, range)
    }

    /// Replaces a range with the contents of a string slice.
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>,
    {
        let promote = match self.cast_mut() {
            StringCastMut::Boxed(mut string) => {
                string.replace_range(range, replace_with);
                return;
            },
            StringCastMut::Inline(string) => {
                let len = string.len();
                let (start, end) = bounds_for(&range, len);
                assert!(end >= start);
                assert!(end <= len);
                assert!(string.is_char_boundary(start));
                assert!(string.is_char_boundary(end));
                let replaced_len = end - start;
                let replace_len = replace_with.len();
                if (len - replaced_len) + replace_len > Mode::MAX_INLINE {
                    let mut string = string.to_string();
                    string.replace_range(range, replace_with);
                    string
                } else {
                    let new_end = start + replace_len;
                    let end_size = len - end;
                    unsafe {
                        let ptr = string.as_mut_slice().as_mut_ptr();
                        ptr.add(end).copy_to(ptr.add(new_end), end_size);
                        ptr.add(start)
                            .copy_from(replace_with.as_bytes().as_ptr().cast(), replace_len);
                        string.set_len(start + replace_len + end_size);
                    }
                    return;
                }
            }
        };
        unsafe {
            self.promote_from(promote);
        }
    }
}

fn bounds_for<R>(range: &R, max_len: usize) -> (usize, usize)
where
    R: RangeBounds<usize>,
{
    let start = match range.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n.checked_add(1).unwrap(),
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&n) => n.checked_add(1).unwrap(),
        Bound::Excluded(&n) => n,
        Bound::Unbounded => max_len,
    };
    (start, end)
}

impl<Mode: SmartStringMode> Default for SmartString<Mode> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Mode: SmartStringMode> AsRef<str> for SmartString<Mode> {
    fn as_ref(&self) -> &str {
        self.deref()
    }
}

impl<Mode: SmartStringMode> AsMut<str> for SmartString<Mode> {
    fn as_mut(&mut self) -> &mut str {
        self.deref_mut()
    }
}

impl<Mode: SmartStringMode> AsRef<[u8]> for SmartString<Mode> {
    fn as_ref(&self) -> &[u8] {
        self.deref().as_bytes()
    }
}

impl<Mode: SmartStringMode> Borrow<str> for SmartString<Mode> {
    fn borrow(&self) -> &str {
        self.deref()
    }
}

impl<Mode: SmartStringMode> BorrowMut<str> for SmartString<Mode> {
    fn borrow_mut(&mut self) -> &mut str {
        self.deref_mut()
    }
}

impl<Mode: SmartStringMode> Index<Range<usize>> for SmartString<Mode> {
    type Output = str;
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.deref()[index]
    }
}

impl<Mode: SmartStringMode> Index<RangeTo<usize>> for SmartString<Mode> {
    type Output = str;
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.deref()[index]
    }
}

impl<Mode: SmartStringMode> Index<RangeFrom<usize>> for SmartString<Mode> {
    type Output = str;
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.deref()[index]
    }
}

impl<Mode: SmartStringMode> Index<RangeFull> for SmartString<Mode> {
    type Output = str;
    fn index(&self, _index: RangeFull) -> &Self::Output {
        self.deref()
    }
}

impl<Mode: SmartStringMode> Index<RangeInclusive<usize>> for SmartString<Mode> {
    type Output = str;
    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        &self.deref()[index]
    }
}

impl<Mode: SmartStringMode> Index<RangeToInclusive<usize>> for SmartString<Mode> {
    type Output = str;
    fn index(&self, index: RangeToInclusive<usize>) -> &Self::Output {
        &self.deref()[index]
    }
}

impl<Mode: SmartStringMode> IndexMut<Range<usize>> for SmartString<Mode> {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self.deref_mut()[index]
    }
}

impl<Mode: SmartStringMode> IndexMut<RangeTo<usize>> for SmartString<Mode> {
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        &mut self.deref_mut()[index]
    }
}

impl<Mode: SmartStringMode> IndexMut<RangeFrom<usize>> for SmartString<Mode> {
    fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut Self::Output {
        &mut self.deref_mut()[index]
    }
}

impl<Mode: SmartStringMode> IndexMut<RangeFull> for SmartString<Mode> {
    fn index_mut(&mut self, _index: RangeFull) -> &mut Self::Output {
        self.deref_mut()
    }
}

impl<Mode: SmartStringMode> IndexMut<RangeInclusive<usize>> for SmartString<Mode> {
    fn index_mut(&mut self, index: RangeInclusive<usize>) -> &mut Self::Output {
        &mut self.deref_mut()[index]
    }
}

impl<Mode: SmartStringMode> IndexMut<RangeToInclusive<usize>> for SmartString<Mode> {
    fn index_mut(&mut self, index: RangeToInclusive<usize>) -> &mut Self::Output {
        &mut self.deref_mut()[index]
    }
}

impl<Mode: SmartStringMode> From<&'_ str> for SmartString<Mode> {
    fn from(string: &'_ str) -> Self {
        if string.len() > Mode::MAX_INLINE {
            Self::from_boxed(unsafe {
                Mode::BoxedString::from_string_unchecked(string.to_string())
            })
        } else {
            Self::from_inline(string.as_bytes().into())
        }
    }
}

impl<Mode: SmartStringMode> From<&'_ String> for SmartString<Mode> {
    fn from(string: &'_ String) -> Self {
        if string.len() > Mode::MAX_INLINE {
            Self::from_boxed(unsafe {
                Mode::BoxedString::from_string_unchecked(string.clone())
            })
        } else {
            Self::from_inline(string.as_bytes().into())
        }
    }
}

impl<Mode: SmartStringMode> From<String> for SmartString<Mode> {
    fn from(string: String) -> Self {
        if string.len() > Mode::MAX_INLINE {
            Self::from_boxed(unsafe {
                Mode::BoxedString::from_string_unchecked(string)
            })
        } else {
            Self::from_inline(string.as_bytes().into())
        }
    }
}

impl<Mode: SmartStringMode> From<Box<str>> for SmartString<Mode> {
    fn from(string: Box<str>) -> Self {
        if string.len() > Mode::MAX_INLINE {
            String::from(string).into()
        } else {
            Self::from(&*string)
        }
    }
}

impl<'a, Mode: SmartStringMode> Extend<&'a str> for SmartString<Mode> {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        for item in iter {
            self.push_str(item);
        }
    }
}

impl<'a, Mode: SmartStringMode> Extend<&'a char> for SmartString<Mode> {
    fn extend<I: IntoIterator<Item = &'a char>>(&mut self, iter: I) {
        for item in iter {
            self.push(*item);
        }
    }
}

impl<Mode: SmartStringMode> Extend<char> for SmartString<Mode> {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<Mode: SmartStringMode> Extend<SmartString<Mode>> for SmartString<Mode> {
    fn extend<I: IntoIterator<Item = SmartString<Mode>>>(&mut self, iter: I) {
        for item in iter {
            self.push_str(&item);
        }
    }
}

impl<Mode: SmartStringMode> Extend<String> for SmartString<Mode> {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        for item in iter {
            self.push_str(&item);
        }
    }
}

impl<'a, Mode: SmartStringMode + 'a> Extend<&'a SmartString<Mode>> for SmartString<Mode> {
    fn extend<I: IntoIterator<Item = &'a SmartString<Mode>>>(&mut self, iter: I) {
        for item in iter {
            self.push_str(item);
        }
    }
}

impl<'a, Mode: SmartStringMode> Extend<&'a String> for SmartString<Mode> {
    fn extend<I: IntoIterator<Item = &'a String>>(&mut self, iter: I) {
        for item in iter {
            self.push_str(item);
        }
    }
}

impl<Mode: SmartStringMode> Add<Self> for SmartString<Mode> {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self::Output {
        self.push_str(&rhs);
        self
    }
}

impl<Mode: SmartStringMode> Add<&'_ Self> for SmartString<Mode> {
    type Output = Self;
    fn add(mut self, rhs: &'_ Self) -> Self::Output {
        self.push_str(rhs);
        self
    }
}

impl<Mode: SmartStringMode> Add<&'_ str> for SmartString<Mode> {
    type Output = Self;
    fn add(mut self, rhs: &'_ str) -> Self::Output {
        self.push_str(rhs);
        self
    }
}

impl<Mode: SmartStringMode> Add<&'_ String> for SmartString<Mode> {
    type Output = Self;
    fn add(mut self, rhs: &'_ String) -> Self::Output {
        self.push_str(rhs);
        self
    }
}

impl<Mode: SmartStringMode> Add<String> for SmartString<Mode> {
    type Output = Self;
    fn add(mut self, rhs: String) -> Self::Output {
        self.push_str(&rhs);
        self
    }
}

impl<Mode: SmartStringMode> Add<SmartString<Mode>> for String {
    type Output = Self;
    fn add(mut self, rhs: SmartString<Mode>) -> Self::Output {
        self.push_str(&rhs);
        self
    }
}

impl<Mode: SmartStringMode> FromIterator<Self> for SmartString<Mode> {
    fn from_iter<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        let mut out = Self::new();
        out.extend(iter.into_iter());
        out
    }
}

impl<Mode: SmartStringMode> FromIterator<String> for SmartString<Mode> {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut out = Self::new();
        out.extend(iter.into_iter());
        out
    }
}

impl<'a, Mode: SmartStringMode + 'a> FromIterator<&'a Self> for SmartString<Mode> {
    fn from_iter<I: IntoIterator<Item = &'a Self>>(iter: I) -> Self {
        let mut out = Self::new();
        out.extend(iter.into_iter());
        out
    }
}

impl<'a, Mode: SmartStringMode> FromIterator<&'a str> for SmartString<Mode> {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Self {
        let mut out = Self::new();
        out.extend(iter.into_iter());
        out
    }
}

impl<'a, Mode: SmartStringMode> FromIterator<&'a String> for SmartString<Mode> {
    fn from_iter<I: IntoIterator<Item = &'a String>>(iter: I) -> Self {
        let mut out = Self::new();
        out.extend(iter.into_iter());
        out
    }
}

impl<Mode: SmartStringMode> FromIterator<char> for SmartString<Mode> {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let mut out = Self::new();
        for ch in iter {
            out.push(ch);
        }
        out
    }
}

impl<Mode: SmartStringMode> FromStr for SmartString<Mode> {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl<Mode: SmartStringMode> Into<String> for SmartString<Mode> {
    /// Unwrap a boxed [`String`][String], or copy an inline string into a new [`String`][String].
    ///
    /// [String]: https://doc.rust-lang.org/std/string/struct.String.html
    fn into(self) -> String {
        match self.cast_into() {
            StringCastInto::Boxed(string) => string.into(),
            StringCastInto::Inline(string) => string.to_string(),
        }
    }
}

impl<Mode: SmartStringMode> PartialEq<str> for SmartString<Mode> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<Mode: SmartStringMode> PartialEq<SmartString<Mode>> for &'_ str {
    fn eq(&self, other: &SmartString<Mode>) -> bool {
        other.eq(*self)
    }
}

impl<Mode: SmartStringMode> PartialEq<SmartString<Mode>> for str {
    fn eq(&self, other: &SmartString<Mode>) -> bool {
        other.eq(self)
    }
}

impl<Mode: SmartStringMode> PartialEq<String> for SmartString<Mode> {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl<Mode: SmartStringMode> PartialEq<SmartString<Mode>> for String {
    fn eq(&self, other: &SmartString<Mode>) -> bool {
        other.eq(self.as_str())
    }
}

impl<Mode: SmartStringMode> PartialEq for SmartString<Mode> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<Mode: SmartStringMode> Eq for SmartString<Mode> {}

impl<Mode: SmartStringMode> PartialOrd<str> for SmartString<Mode> {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.as_str().partial_cmp(other)
    }
}

impl<Mode: SmartStringMode> PartialOrd for SmartString<Mode> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.partial_cmp(other.as_str())
    }
}

impl<Mode: SmartStringMode> Ord for SmartString<Mode> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<Mode: SmartStringMode> Hash for SmartString<Mode> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl<Mode: SmartStringMode> Debug for SmartString<Mode> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Debug::fmt(self.as_str(), f)
    }
}

impl<Mode: SmartStringMode> Display for SmartString<Mode> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Display::fmt(self.as_str(), f)
    }
}

impl<Mode: SmartStringMode> Write for SmartString<Mode> {
    fn write_str(&mut self, string: &str) -> Result<(), Error> {
        self.push_str(string);
        Ok(())
    }
}

#[cfg(any(test, feature = "test"))]
#[allow(missing_docs)]
pub mod test;
