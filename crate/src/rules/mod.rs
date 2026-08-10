//! Rule implementations, one per module.
//!
//! Each rule is added as its corresponding issue lands. Modules are
//! declared here as they come online.

pub(crate) mod align_colons;
pub(crate) mod align_comments;
pub(crate) mod align_comparisons;
pub(crate) mod align_equals;
pub(crate) mod align_imports;
pub(crate) mod align_match_case;
pub(crate) mod alphabetize_siblings;
pub(crate) mod band_constants;
pub(crate) mod bare_imports;
pub(crate) mod expand_docstrings;
pub(crate) mod frame_docstrings;
pub(crate) mod group_imports;
pub(crate) mod line_overflow;
pub(crate) mod miscased_constants;
pub(crate) mod modernize_annotations;
pub(crate) mod normalize_comment_spacing;
pub(crate) mod normalize_comparisons;
pub(crate) mod normalize_literals;
pub(crate) mod prefer_fstring;
pub(crate) mod prune_inert_imports;
pub(crate) mod reassigned_constants;
pub(crate) mod reflow_calls;
pub(crate) mod reflow_collections;
pub(crate) mod reflow_imports;
pub(crate) mod reflow_signatures;
pub(crate) mod restated_types;
pub(crate) mod shed_backslash_continuations;
pub(crate) mod shed_parentheses;
pub(crate) mod shed_redundant_base;
pub(crate) mod shed_super_args;
pub(crate) mod signature_annotations;
pub(crate) mod simplify_comprehensions;
pub(crate) mod single_use_variables;
pub(crate) mod space_statements;
pub(crate) mod stack_adjacent_strings;
pub(crate) mod stack_method_chains;
pub(crate) mod step_narration;
pub(crate) mod strip_none_return;
pub(crate) mod strip_stranded_padding;
pub(crate) mod strip_trailing_commas;
pub(crate) mod unsorted_positionals;
pub(crate) mod wrap_docstrings;
