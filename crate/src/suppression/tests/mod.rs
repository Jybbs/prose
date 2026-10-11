//! Suppression tests, one file per surface they cover, with the line
//! helper they share held here.

use ruff_source_file::OneIndexed;

mod map;
mod parse;

/// The one-indexed line `zero_indexed` rows from the top.
fn line(zero_indexed: usize) -> OneIndexed {
    OneIndexed::from_zero_indexed(zero_indexed)
}
