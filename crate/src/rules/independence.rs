//! The pairs of rules whose edits splice into one buffer and parse
//! once, read off the subset probe's evidence and what each rule's
//! `apply` measures.

use super::registry::{KNOWN_IDS, PIPELINE_DEPENDENCIES, precedes, slug_bytes_equal, slug_index};

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
    ("alphabetize-siblings", &["shed-redundant-base"]),
    (
        "space-statements",
        &[
            "shed-redundant-base",
            "stack-adjacent-strings",
            "align-match-case",
        ],
    ),
    (
        "align-imports",
        &[
            "shed-redundant-base",
            "stack-adjacent-strings",
            "align-match-case",
        ],
    ),
    (
        "align-colons",
        &["shed-redundant-base", "space-statements", "align-imports"],
    ),
    (
        "wrap-docstrings",
        &[
            "shed-redundant-base",
            "align-match-case",
            "space-statements",
            "align-imports",
        ],
    ),
    (
        "align-equals",
        &[
            "shed-redundant-base",
            "space-statements",
            "align-imports",
            "wrap-docstrings",
        ],
    ),
    (
        "align-comparisons",
        &[
            "prune-inert-imports",
            "shed-redundant-base",
            "frame-docstrings",
            "expand-docstrings",
            "group-imports",
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
        &["shed-redundant-base", "wrap-docstrings"],
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
// earlier rule seated ahead of it, and none the later rule reaches
// through its dependency column, directly or through the column of a
// rule that column names.
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
                !reaches(seat, earlier[j]),
                "a rule cannot share a splice with a rule it runs behind",
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
        .any(|(slug, shared)| *slug == later && shared.contains(&earlier))
}

/// Whether the rule at `seat` reaches `slug` through its dependency
/// column, directly or through the column of a rule that column names.
/// The const counterpart of [`runs_behind`](super::runs_behind).
pub(super) const fn reaches(seat: usize, slug: &str) -> bool {
    let mut seen = [false; KNOWN_IDS.len()];
    let mut pending = [0usize; KNOWN_IDS.len()];
    let mut depth = 1;
    pending[0] = seat;
    seen[seat] = true;
    while depth > 0 {
        depth -= 1;
        let column = PIPELINE_DEPENDENCIES[pending[depth]];
        let mut i = 0;
        while i < column.len() {
            if slug_bytes_equal(column[i].as_bytes(), slug.as_bytes()) {
                return true;
            }
            if let Some(next) = slug_index(column[i])
                && !seen[next]
            {
                seen[next] = true;
                pending[depth] = next;
                depth += 1;
            }
            i += 1;
        }
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
}
