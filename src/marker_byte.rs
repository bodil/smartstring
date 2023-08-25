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

#[derive(Clone, Copy, Debug)]
pub(crate) struct Marker(NonZeroU8);

impl Marker {
    #[inline(always)]
    const fn assemble(discriminant: Discriminant, data: u8) -> NonZeroU8 {
        debug_assert!(data < 0x40);

        #[allow(unsafe_code)]
        unsafe { NonZeroU8::new_unchecked(0x80 | (data << 1) | discriminant.bit()) } // SAFETY: (0x80 | x) != 0 is guaranteed for all x
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
        (self.0.get() & 0x7f) >> 1
    }

    #[inline(always)]
    pub(crate) fn set_data(&mut self, byte: u8) {
        self.0 = Self::assemble(self.discriminant(), byte);
    }
}
