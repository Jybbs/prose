//! Prose is an opinionated Python code formatter.
//!
//! See the project README and the approved plan for design rationale.

#![cfg_attr(not(feature = "native"), allow(dead_code, unused_imports))]

#[cfg(feature = "native")]
pub(crate) mod cache;
#[cfg(feature = "native")]
pub mod cli;
pub mod config;
pub mod diagnostics;
mod file_uri;
pub mod pipeline;
mod primitives;
pub mod rule;
mod rules;
#[cfg(feature = "native")]
mod server;
pub mod source;
pub(crate) mod suppression;
#[cfg(test)]
mod testing;
#[cfg(feature = "native")]
mod walker;

pub use primitives::binding::BindingAnalysis;
