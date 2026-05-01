//! Prose is an opinionated Python code formatter.
//!
//! See the project README and the approved plan for design rationale.

pub mod cli;
pub mod config;
pub mod pipeline;
mod primitives;
mod rules;
pub mod source;
#[cfg(test)]
mod test_support;
mod walker;
