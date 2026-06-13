//! Rule implementations, one per module.
//!
//! Each rule is added as its corresponding issue lands. Modules are
//! declared here as they come online.

pub(crate) mod align_colons;
pub(crate) mod align_comparisons;
pub(crate) mod align_equals;
pub(crate) mod align_imports;
pub(crate) mod align_match_case;
pub(crate) mod alphabetize;
pub(crate) mod bare_imports;
pub(crate) mod blank_lines;
pub(crate) mod call_layout;
pub(crate) mod collection_layout;
pub(crate) mod docstring_expand;
pub(crate) mod docstring_frame;
pub(crate) mod docstring_wrap;
pub(crate) mod import_layout;
pub(crate) mod legacy_union_syntax;
pub(crate) mod reassigned_constants;
pub(crate) mod signature_layout;
pub(crate) mod single_use_variables;
pub(crate) mod step_narration;
pub(crate) mod strip_align_padding;
pub(crate) mod strip_trailing_commas;
pub(crate) mod unsorted_parameters;
pub(crate) mod unused_future_annotations;
