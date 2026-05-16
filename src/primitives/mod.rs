//! Shared primitives used across rule implementations. `aligner`
//! emits alignment edits for groups sharing a token. `binding`
//! walks the AST once and records every name's writes and reads for
//! consumers that ask scope-aware questions. `colon_targets`
//! constructs alignment members at every `:` context the alignment
//! and singleton rules consume. `docstring` collects every PEP 257
//! docstring `StringLiteral` reachable from the module body. `edit`
//! shapes replacement text into minimal-range edits and folds inline
//! edits into source slices. `orderer` reorders sibling AST nodes by
//! a key function while preserving attached comments and inter-section
//! content.

pub(crate) mod aligner;
pub(crate) mod binding;
pub(crate) mod colon_targets;
pub(crate) mod docstring;
pub(crate) mod edit;
pub(crate) mod orderer;

/// PEP 8 indent step in spaces, the depth one nested level adds.
pub(crate) const INDENT_STEP: usize = 4;
