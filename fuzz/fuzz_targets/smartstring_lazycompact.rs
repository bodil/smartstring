// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![no_main]

use libfuzzer_sys::fuzz_target;
use smartstring::{
    test::{test_everything, Action, Constructor},
    LazyCompact,
};

fuzz_target!(|input: (Constructor, Vec<Action>)| {
    let (constructor, actions) = input;
    test_everything::<LazyCompact>(constructor, actions);
});
