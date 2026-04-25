//! Shared primitives used across rule implementations. `aligner`
//! emits alignment edits for groups sharing a token. `locator` lifts
//! position helpers over `Source`.

pub mod aligner;
pub mod locator;
