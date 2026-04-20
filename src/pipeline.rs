//! Runs the enabled rules against a source file in deterministic order.
//!
//! Order matters: alignment rules run last so that any earlier rewrites
//! (alphabetization, collection expansion) have already settled before
//! the aligner computes padding widths.
