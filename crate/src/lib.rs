//! Prose is an opinionated Python code formatter.
//!
//! See the project README and the approved plan for design rationale.

#![cfg_attr(not(feature = "native"), allow(dead_code, unused_imports))]

pub mod config;
pub mod diagnostics;
pub mod findings;
pub mod pipeline;
mod primitives;
pub mod rules;
pub mod source;
pub(crate) mod suppression;
#[cfg(test)]
mod testing;

#[cfg(feature = "native")]
pub(crate) mod cache;
#[cfg(feature = "native")]
pub mod cli;
#[cfg(feature = "native")]
mod file_uri;
#[cfg(feature = "native")]
mod server;
#[cfg(feature = "native")]
mod unstable;
#[cfg(feature = "native")]
mod walker;

pub use primitives::binding::BindingAnalysis;
