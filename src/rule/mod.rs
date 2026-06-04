//! Rule abstraction, identifier types, and the registry that ties
//! concrete rule structs to the pipeline orchestrator.
//!
//! Each concrete rule lives under `crate::rules`. The [`Rule`] trait
//! and the [`RuleId`] newtype defined here are the canonical handles.
//! The `register_rules!` macro emits [`KNOWN_IDS`], [`RuleConfigs`],
//! [`Pipeline::for_rule`], [`Pipeline::with_defaults`], and
//! [`Pipeline::with_filters`] from a registry table.

mod id;
mod registry;
mod slug;
mod trait_;

pub use id::{ParseRuleIdError, RuleId};
pub(crate) use registry::KNOWN_IDS;
pub use registry::RuleConfigs;
pub(crate) use trait_::Rule;
