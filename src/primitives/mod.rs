//! Shared primitives used across rule implementations. `aligner`
//! emits alignment edits for groups sharing a token. `colon_targets`
//! constructs alignment members at every `:` context the alignment
//! and singleton rules consume. `locator` lifts position helpers over
//! `Source`. `orderer` reorders sibling AST nodes by a key function
//! while preserving attached comments and inter-section content.

pub mod aligner;
pub mod colon_targets;
pub mod locator;
pub mod orderer;
