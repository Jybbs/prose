//! Wrapper around source text plus `TextRange` lookup helpers.
//!
//! Owns the original source buffer and exposes range-based slicing for
//! the token-level whitespace manipulation that alignment rules require.
//! Every rule sees the source through this wrapper so splicing edits
//! compose correctly.
