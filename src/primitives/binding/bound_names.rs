//! Resolution of the module-scope name an `import` alias binds.

use ruff_python_ast::{Alias, Identifier};

/// The module-scope name a bare `import a.b` alias binds: its `asname`,
/// or the top-level segment of the dotted path.
pub(crate) fn bare_import_bound_name(alias: &Alias) -> &str {
    alias
        .asname
        .as_ref()
        .map_or_else(|| top_level_module(alias.name.as_str()), Identifier::as_str)
}

/// The name a `from m import x` alias binds: its `asname`, or the
/// imported name itself.
pub(crate) fn from_import_bound_name(alias: &Alias) -> &str {
    alias.asname.as_ref().unwrap_or(&alias.name).as_str()
}

/// Returns the segment of `dotted` before the first `.`. Matches
/// Python's `import a.b.c` shape, which binds `a` rather than the
/// full dotted path.
pub(crate) fn top_level_module(dotted: &str) -> &str {
    dotted.split_once('.').map_or(dotted, |(head, _)| head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_module_returns_first_segment() {
        assert_eq!(top_level_module("a"), "a");
        assert_eq!(top_level_module("a.b"), "a");
        assert_eq!(top_level_module("a.b.c"), "a");
        assert_eq!(top_level_module(""), "");
    }
}
