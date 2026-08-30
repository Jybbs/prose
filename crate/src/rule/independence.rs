//! The pairs of rules whose edits splice into one buffer and parse
//! once, read off the subset probe's evidence and what each rule's
//! `apply` measures.

use super::{PIPELINE_DEPENDENCIES, precedes, slug_bytes_equal, slug_index};

/// The earlier rules each rule shares a splice and a parse with, keyed
/// by the later rule's slug. A pair is listed only where the subset
/// probe found the two rules editing a standard-library file together
/// with the batched splice matching the fold on every such file at
/// every line length, and where a reading of the later rule's `apply`
/// finds nothing it measures among what the earlier rule rewrites,
/// meaning the text a column is derived from, the adjacency of the
/// rows a run spans, a statement's position, a name binding, or a
/// docstring's rows. A row's fit against the budget is left to the
/// probe, so a rule measuring only that shares a splice with a rule
/// rewriting the row's value side.
const INDEPENDENCE: &[(&str, &[&str])] = &[
    ("normalize-literals", &["shed-backslash-continuations"]),
    (
        "strip-none-return",
        &[
            "shed-backslash-continuations",
            "normalize-literals",
            "prune-inert-imports",
        ],
    ),
    (
        "strip-trailing-commas",
        &[
            "shed-backslash-continuations",
            "normalize-literals",
            "prune-inert-imports",
            "strip-none-return",
        ],
    ),
    (
        "normalize-comparisons",
        &[
            "shed-backslash-continuations",
            "normalize-literals",
            "prune-inert-imports",
            "strip-none-return",
            "strip-trailing-commas",
        ],
    ),
    ("reflow-parentheses", &["prune-inert-imports"]),
    (
        "shed-redundant-base",
        &[
            "shed-backslash-continuations",
            "normalize-literals",
            "prune-inert-imports",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
        ],
    ),
    (
        "simplify-comprehensions",
        &["strip-none-return", "shed-redundant-base"],
    ),
    (
        "frame-docstrings",
        &[
            "shed-backslash-continuations",
            "normalize-literals",
            "prune-inert-imports",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
            "shed-redundant-base",
            "simplify-comprehensions",
        ],
    ),
    (
        "expand-docstrings",
        &[
            "shed-backslash-continuations",
            "normalize-literals",
            "prune-inert-imports",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
            "shed-redundant-base",
            "simplify-comprehensions",
        ],
    ),
    (
        "group-imports",
        &[
            "normalize-literals",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
            "shed-redundant-base",
            "simplify-comprehensions",
            "frame-docstrings",
            "expand-docstrings",
        ],
    ),
    (
        "shed-super-args",
        &[
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
        ],
    ),
    (
        "stack-method-chains",
        &[
            "prune-inert-imports",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
        ],
    ),
    (
        "reflow-calls",
        &[
            "prune-inert-imports",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
        ],
    ),
    (
        "reflow-signatures",
        &[
            "prune-inert-imports",
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
        ],
    ),
    (
        "reflow-collections",
        &[
            "prune-inert-imports",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
        ],
    ),
    (
        "stack-adjacent-strings",
        &["frame-docstrings", "expand-docstrings"],
    ),
    (
        "align-match-case",
        &[
            "strip-none-return",
            "strip-trailing-commas",
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
            "reflow-signatures",
        ],
    ),
    (
        "reflow-imports",
        &[
            "normalize-literals",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
            "shed-redundant-base",
            "simplify-comprehensions",
            "frame-docstrings",
            "expand-docstrings",
            "shed-super-args",
            "stack-method-chains",
            "reflow-calls",
            "reflow-signatures",
            "reflow-collections",
        ],
    ),
    (
        "band-constants",
        &[
            "strip-none-return",
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "reflow-signatures",
        ],
    ),
    (
        "alphabetize-siblings",
        &["strip-none-return", "shed-redundant-base"],
    ),
    (
        "space-statements",
        &[
            "normalize-literals",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "shed-redundant-base",
            "simplify-comprehensions",
            "frame-docstrings",
            "expand-docstrings",
            "shed-super-args",
            "stack-method-chains",
            "reflow-calls",
            "reflow-signatures",
            "reflow-collections",
            "stack-adjacent-strings",
            "align-match-case",
        ],
    ),
    (
        "align-imports",
        &[
            "normalize-literals",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
            "shed-redundant-base",
            "simplify-comprehensions",
            "frame-docstrings",
            "expand-docstrings",
            "shed-super-args",
            "stack-method-chains",
            "reflow-calls",
            "reflow-signatures",
            "reflow-collections",
            "stack-adjacent-strings",
            "align-match-case",
        ],
    ),
    (
        "align-colons",
        &[
            "strip-none-return",
            "normalize-comparisons",
            "shed-redundant-base",
            "shed-super-args",
            "stack-method-chains",
            "space-statements",
            "align-imports",
        ],
    ),
    (
        "wrap-docstrings",
        &[
            "shed-backslash-continuations",
            "prune-inert-imports",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
            "shed-redundant-base",
            "simplify-comprehensions",
            "group-imports",
            "shed-super-args",
            "stack-method-chains",
            "reflow-calls",
            "reflow-signatures",
            "reflow-collections",
            "stack-adjacent-strings",
            "align-match-case",
            "reflow-imports",
            "band-constants",
            "alphabetize-siblings",
            "space-statements",
            "align-imports",
        ],
    ),
    (
        "align-equals",
        &[
            "strip-none-return",
            "normalize-comparisons",
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "space-statements",
            "align-imports",
            "wrap-docstrings",
        ],
    ),
    (
        "align-comparisons",
        &[
            "prune-inert-imports",
            "strip-none-return",
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
            "reflow-signatures",
            "reflow-imports",
            "band-constants",
            "alphabetize-siblings",
            "space-statements",
            "align-imports",
            "wrap-docstrings",
        ],
    ),
    (
        "strip-stranded-padding",
        &[
            "prune-inert-imports",
            "strip-none-return",
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
            "reflow-imports",
            "band-constants",
            "space-statements",
            "wrap-docstrings",
        ],
    ),
    (
        "normalize-comment-spacing",
        &[
            "shed-backslash-continuations",
            "normalize-literals",
            "prune-inert-imports",
            "strip-none-return",
            "strip-trailing-commas",
            "normalize-comparisons",
            "reflow-parentheses",
            "shed-redundant-base",
            "simplify-comprehensions",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
            "shed-super-args",
            "stack-method-chains",
            "reflow-calls",
            "reflow-signatures",
            "reflow-collections",
            "align-match-case",
            "reflow-imports",
            "band-constants",
            "alphabetize-siblings",
            "space-statements",
            "align-imports",
            "align-colons",
            "wrap-docstrings",
            "align-equals",
            "align-comparisons",
            "strip-stranded-padding",
        ],
    ),
];

// Asserts each independence entry names a registered later rule, each
// earlier rule seated ahead of it, and none its dependency column names.
const _: () = {
    let mut i = 0;
    while i < INDEPENDENCE.len() {
        let (later, earlier) = INDEPENDENCE[i];
        let Some(seat) = slug_index(later) else {
            panic!("an independence entry names an unregistered rule");
        };
        let mut j = 0;
        while j < earlier.len() {
            assert!(
                precedes(earlier[j], later),
                "an independent rule must be registered ahead of the rule sharing its splice",
            );
            assert!(
                !names_slug(PIPELINE_DEPENDENCIES[seat], earlier[j]),
                "a rule cannot share a splice with a rule its dependency column names",
            );
            j += 1;
        }
        i += 1;
    }
};

/// Whether `later` shares a splice and a parse with `earlier`, `false`
/// for an unknown slug on either side.
pub fn independent(later: &str, earlier: &str) -> bool {
    INDEPENDENCE
        .iter()
        .find(|(slug, _)| *slug == later)
        .is_some_and(|(_, shared)| shared.contains(&earlier))
}

/// Returns `true` when `slugs` holds `slug`.
const fn names_slug(slugs: &[&str], slug: &str) -> bool {
    let mut i = 0;
    while i < slugs.len() {
        if slug_bytes_equal(slugs[i].as_bytes(), slug.as_bytes()) {
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("strip-trailing-commas", "normalize-literals", true)]
    #[case("normalize-literals", "strip-trailing-commas", false)]
    #[case("align-equals", "align-colons", false)]
    #[case("align-equals", "not-a-rule", false)]
    fn independent_reads_the_pair_in_registry_order(
        #[case] later: &str,
        #[case] earlier: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(independent(later, earlier), expected);
    }

    #[rstest]
    #[case(&["align-colons", "align-equals"], "align-equals", true)]
    #[case(&["align-colons", "align-equals"], "align-imports", false)]
    #[case(&[], "align-equals", false)]
    fn names_slug_matches_only_a_listed_slug(
        #[case] slugs: &[&str],
        #[case] slug: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(names_slug(slugs, slug), expected);
    }
}
