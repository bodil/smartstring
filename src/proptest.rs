//! `proptest` strategies (requires the `proptest` feature flag).

use crate::{SmartString, SmartStringMode};
use proptest::proptest;
use proptest::strategy::{BoxedStrategy, Strategy};
use proptest::string::Error;

/// Creates a strategy which generates [`SmartString`][SmartString]s matching the given regular expression.
///
/// [SmartString]: ../struct.SmartString.html
pub fn string_regex<Mode: SmartStringMode>(
    regex: &str,
) -> Result<BoxedStrategy<SmartString<Mode>>, Error>
where
    Mode: 'static,
{
    proptest::string::string_regex(regex).map(|g| g.prop_map(SmartString::from).boxed())
}

proptest! {
    #[test]
    fn strategy(string in string_regex(".+").unwrap()) {
        assert!(!SmartString::<crate::LazyCompact>::is_empty(&string));
    }
}
