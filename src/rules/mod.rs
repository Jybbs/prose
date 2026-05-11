//! Rule implementations, one per module.
//!
//! Each rule is added as its corresponding issue lands. Modules are
//! declared here as they come online.

pub(crate) mod align_colons;
pub(crate) mod align_equals;
pub(crate) mod align_imports;
pub(crate) mod alphabetize;
pub(crate) mod blank_lines;
pub(crate) mod collection_layout;
pub(crate) mod match_case_align;
pub(crate) mod singleton_rule;
pub(crate) mod strip_trailing_commas;
