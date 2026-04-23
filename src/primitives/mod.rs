//! Shared primitives used across rule implementations.
//!
//! `aligner` computes padding widths for any group of lines that share
//! an alignable token. `orderer` reshuffles sibling AST nodes and
//! regenerates source for the alphabetization rules.

pub mod aligner;
