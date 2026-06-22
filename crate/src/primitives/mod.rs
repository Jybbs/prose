//! Shared primitives used across rule implementations.

pub(crate) mod aligner;
pub(crate) mod binding;
pub(crate) mod call_keywords;
pub(crate) mod colon_targets;
pub(crate) mod comments;
pub(crate) mod docstring;
pub(crate) mod edit;
pub(crate) mod equal_targets;
pub(crate) mod imports;
pub(crate) mod inline;
pub(crate) mod layout;
pub(crate) mod orderer;
pub(crate) mod params;
pub(crate) mod range;
pub(crate) mod scope;
pub(crate) mod sections;

/// PEP 8 indent step in spaces, the depth one nested level adds.
pub(crate) const INDENT_STEP: usize = 4;
