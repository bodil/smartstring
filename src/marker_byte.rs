// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use core::num::NonZeroU8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Discriminant {
    Boxed,
    Inline,
}

impl Discriminant {
    #[inline(always)]
    pub(crate) const fn from_bit(bit: bool) -> Self {
        if bit {
            Self::Inline
        } else {
            Self::Boxed
        }
    }

    #[inline(always)]
    const fn bit(self) -> u8 {
        match self {
            Self::Boxed => 0,
            Self::Inline => 1,
        }
    }
}

/// We now use this type to facilitate Option size optimization.
/// The low two bits are used to determine both the discriminant and the None state.
///
/// 00000000 - None
/// xxxxxx01 - unused
/// xxxxxx10 - BoxedString
/// xxxxxx11 - InlineString
///
/// BoxedString now uses TaggedPtr to ensure the low two bits form the 10 pattern.
/// This guarantees the in-memory NonZeroU8 value is always in a valid state and that it matches the
/// tagging convention of Marker.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Marker(NonZeroU8);

impl Marker {
    #[inline(always)]
    const fn assemble(discriminant: Discriminant, data: u8) -> NonZeroU8 {
        debug_assert!(data < 0x40);

        #[allow(unsafe_code)]
        unsafe { NonZeroU8::new_unchecked((data << 2) | 2 | discriminant.bit()) } // SAFETY: (2 | x) != 0 is guaranteed for all x
    }

    #[inline(always)]
    pub(crate) const fn empty() -> Self {
        Self(Self::assemble(Discriminant::Inline, 0))
    }

    #[inline(always)]
    pub(crate) const fn new_inline(data: u8) -> Self {
        Self(Self::assemble(Discriminant::Inline, data))
    }

    #[inline(always)]
    pub(crate) const fn discriminant(self) -> Discriminant {
        Discriminant::from_bit(self.0.get() & 0x01 != 0)
    }

    #[inline(always)]
    pub(crate) const fn data(self) -> u8 {
        self.0.get() >> 2
    }

    #[inline(always)]
    pub(crate) fn set_data(&mut self, byte: u8) {
        self.0 = Self::assemble(self.discriminant(), byte);
    }
}
