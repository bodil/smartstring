# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/) and this project
adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### ADDED

-   `SmartString` now implements `FromIterator<char>`.
-   Support for [`serde`](https://serde.rs/) behind the `serde` feature flag.
-   Support for [`arbitrary`](https://crates.io/crates/arbitrary) behind the `arbitrary` feature
    flag.
-   Support for [`proptest`](https://crates.io/crates/proptest) behind the `proptest` feature flag.

## [0.2.2] - 2020-07-05

### FIXED

-   Calling `shrink_to_fit()` on a string with `LazyCompact` layout will now inline it and
    deallocate the heap allocation if the string is short enough to be inlined.

## [0.2.1] - 2020-07-04

### FIXED

-   The type alias `smartstring::alias::String` was incorrectly pointing at the `Compact` variant.
    It is now pointing at `LazyCompact`, as the documentation describes.

## [0.2.0] - 2020-07-04

### REMOVED

-   The `Prefixed` variant has been removed, as it comes with significant code complexity for very
    dubious gains.

### CHANGED

-   The type alias `smartstring::alias::String` now refers to `LazyCompact` instead of `Compact`,
    the idea being that the obvious drop-in replacement for `String` shouldn't have any unexpected
    performance differences, which `Compact` can have because it aggressively re-inlines strings to
    keep them as local as possible. `LazyCompact` instead heap allocates once when the string is in
    excess of the inline capacity and keeps the allocation from then on, so there are no surprises.

### ADDED

-   There's a new layout variant, `LazyCompact`, which works like `Compact` except it never
    re-inlines strings once they have been moved to the heap.
-   As the alias `String` has changed, there is now a new type alias
    `smartstring::alias::CompactString`, referring to strings with `Compact` layout.

### FIXED

-   Fixed a bug where `SmartString::drain()` would remove twice the drained content from the string.

## [0.1.0] - 2020-05-15

Initial release.
