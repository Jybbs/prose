//! Shared primitives used across rule implementations.

pub(crate) mod alias;
pub(crate) mod aligner;
pub(crate) mod binding;
pub(crate) mod blanks;
pub(crate) mod call_keywords;
pub(crate) mod colon_targets;
pub(crate) mod comments;
pub(crate) mod comparison;
pub(crate) mod constructor;
pub(crate) mod decorator;
pub(crate) mod docstring;
pub(crate) mod edit;
pub(crate) mod effect;
pub(crate) mod equal_targets;
pub(crate) mod fracture;
pub(crate) mod imports;
pub(crate) mod inline;
pub(crate) mod layout;
pub(crate) mod orderer;
pub(crate) mod params;
pub(crate) mod quoting;
pub(crate) mod range;
pub(crate) mod reserve;
pub(crate) mod scope;
pub(crate) mod sections;
pub(crate) mod slots;
pub(crate) mod tiering;
pub(crate) mod tokens;
pub(crate) mod walk;

/// PEP 8 indent step in spaces, the depth one nested level adds.
pub(crate) const INDENT_STEP: usize = 4;

/// Inserts `item` into `vec` at the slot keeping it ascending by
/// `key`, before any element whose key compares equal.
pub(crate) fn insert_sorted_by_key<T, K: Ord>(vec: &mut Vec<T>, item: T, key: impl Fn(&T) -> K) {
    let slot = vec.partition_point(|existing| key(existing) < key(&item));
    vec.insert(slot, item);
}
