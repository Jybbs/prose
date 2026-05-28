//! Prose is an opinionated Python code formatter.
//!
//! See the project README and the approved plan for design rationale.

pub(crate) mod cache;
pub mod cli;
pub mod config;
pub mod diagnostics;
pub mod pipeline;
mod primitives;
pub mod rule;
mod rules;
pub mod source;
pub(crate) mod suppression;
#[cfg(test)]
mod test_support;
mod walker;

pub use primitives::binding::BindingAnalysis;
