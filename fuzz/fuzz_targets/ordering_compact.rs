#![no_main]

use libfuzzer_sys::fuzz_target;
use smartstring::{test::test_ordering, Compact};

type Input = (String, String);

fuzz_target!(|input: Input| {
    let (left, right) = input;
    test_ordering::<Compact>(left, right);
});
