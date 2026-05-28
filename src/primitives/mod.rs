//! Shared primitives used across rule implementations.

pub(crate) mod aligner;
pub(crate) mod binding;
pub(crate) mod colon_targets;
pub(crate) mod docstring;
pub(crate) mod edit;
pub(crate) mod imports;
pub(crate) mod orderer;

/// PEP 8 indent step in spaces, the depth one nested level adds.
pub(crate) const INDENT_STEP: usize = 4;
