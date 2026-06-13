//! Prose is an opinionated Python code formatter.
//!
//! See the project README and the approved plan for design rationale.

pub(crate) mod cache;
pub mod cli;
pub mod config;
pub mod diagnostics;
mod file_uri;
pub mod pipeline;
mod primitives;
pub mod rule;
mod rules;
mod server;
pub mod source;
pub(crate) mod suppression;
#[cfg(test)]
mod testing;
mod walker;

pub use primitives::binding::BindingAnalysis;
