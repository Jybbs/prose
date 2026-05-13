//! Rule implementations, one per module.
//!
//! Each rule is added as its corresponding issue lands. Modules are
//! declared here as they come online.

pub(crate) mod align_colons;
pub(crate) mod align_equals;
pub(crate) mod align_imports;
pub(crate) mod alphabetize;
pub(crate) mod bare_import_allowlist;
pub(crate) mod blank_lines;
pub(crate) mod collection_layout;
pub(crate) mod docstring_wrap;
pub(crate) mod legacy_union_syntax;
pub(crate) mod loose_constants;
pub(crate) mod match_case_align;
pub(crate) mod multi_line_docstrings;
pub(crate) mod no_single_line_docstrings;
pub(crate) mod no_step_narration;
pub(crate) mod single_use_variables;
pub(crate) mod singleton_rule;
pub(crate) mod strip_trailing_commas;
pub(crate) mod unused_future_annotations;
