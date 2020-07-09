// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{boxed::StringReference, inline::InlineString, SmartStringMode};

pub(crate) enum StringCast<'a, Mode: SmartStringMode> {
    Boxed(&'a Mode::BoxedString),
    Inline(&'a InlineString<Mode>),
}

pub(crate) enum StringCastMut<'a, Mode: SmartStringMode> {
    Boxed(StringReference<'a, Mode>),
    Inline(&'a mut InlineString<Mode>),
}

pub(crate) enum StringCastInto<Mode: SmartStringMode> {
    Boxed(Mode::BoxedString),
    Inline(InlineString<Mode>),
}

//Same as transmute, except it doesn't check for same size
//This should be replaced when it's possible to constrain the sizes of generic associated types
pub(crate) unsafe fn please_transmute<A, B>(from: A) -> B {
    let ret = std::mem::transmute_copy(&from);
    std::mem::forget(from);
    ret
}
