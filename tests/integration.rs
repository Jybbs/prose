//! Integration tests exercising each rule against golden-file fixtures.
//!
//! Fixtures live under `tests/fixtures/<rule>/<case>.input.py` with the
//! matching `.expected.py` alongside. Assertions run through `insta` so
//! snapshot diffs are reviewable with `cargo insta review`.
