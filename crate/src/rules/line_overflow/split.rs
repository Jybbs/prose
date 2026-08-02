//! The parenthesized adjacent-literal form an over-budget string
//! literal could take, rendered as a display-only edit. Every cut opens
//! at the first character after an interior whitespace run, so the parts
//! rejoin into the value the literal already held.

use std::iter;

use ruff_diagnostics::Edit;
use ruff_python_ast::StringLiteral;
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{
        inline::whitespace_runs,
        layout::{Separator, explode_parens, item_indent, pack},
    },
    source::Source,
};

/// The display-only edit replacing `lit` with the parenthesized
/// adjacent-literal form, each part filled against the columns its
/// indent leaves inside `cap`. `None` when `lit` carries no interior
/// whitespace to cut at.
pub(super) fn concatenation(source: &Source, lit: &StringLiteral, cap: usize) -> Option<Edit> {
    let inner = lit.content_range();
    let indent = source.line_indent_width(lit.start());
    let opener = source.slice(TextRange::new(lit.start(), inner.start()));
    let closer = source.slice(TextRange::new(inner.end(), lit.end()));
    let budget = cap.saturating_sub(item_indent(indent) + opener.width() + closer.width());
    let parts = concatenated_parts(source.slice(inner), budget)?;
    Some(Edit::range_replacement(
        explode_parens(
            source.newline_str(),
            indent,
            parts.len(),
            |out, i| {
                out.push_str(opener);
                out.push_str(parts[i]);
                out.push_str(closer);
            },
            Separator::None,
        ),
        lit.range(),
    ))
}

/// True when `lit` carries an interior whitespace run, the break a
/// split would open at whether or not one is needed.
pub(super) fn has_interior_break(source: &Source, lit: &StringLiteral) -> bool {
    split_points(source.slice(lit.content_range()))
        .next()
        .is_some()
}

/// Splits `content` into the two or more parts an adjacent-literal form
/// carries, filling each to `budget` where the cuts allow. `None` when
/// `content` carries no interior whitespace and when it fits `budget`
/// whole, neither of which leaves a split to make.
fn concatenated_parts(content: &str, budget: usize) -> Option<Vec<&str>> {
    let bounds: Vec<usize> = iter::once(0)
        .chain(split_points(content))
        .chain(iter::once(content.len()))
        .collect();
    let widths: Vec<usize> = bounds
        .windows(2)
        .map(|pair| content[pair[0]..pair[1]].width())
        .collect();
    let lines = pack(&widths, 0, 0, budget);
    (lines.len() > 1).then(|| {
        lines
            .into_iter()
            .map(|line| &content[bounds[line.start]..bounds[line.end]])
            .collect()
    })
}

/// The byte offsets inside `content` where a part may open, each the
/// first character after an interior whitespace run.
fn split_points(content: &str) -> impl Iterator<Item = usize> {
    whitespace_runs(content)
        .filter(move |&(begin, len)| begin > 0 && begin + len < content.len())
        .map(|(begin, len)| begin + len)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::two_parts_at_a_generous_budget("the quick brown fox", 10, &["the quick ", "brown fox"])]
    #[case::one_word_per_part_at_a_tight_budget("the quick brown fox", 4, &["the ", "quick ", "brown ", "fox"])]
    #[case::doubled_runs_travel_with_the_part_they_close("keeps  a  doubled  run", 12, &["keeps  a  ", "doubled  run"])]
    #[case::leading_whitespace_opens_no_part_of_its_own("  leading run held", 14, &["  leading run ", "held"])]
    #[case::an_over_budget_token_stands_alone("aa bbbbbbbbbbbb cc", 4, &["aa ", "bbbbbbbbbbbb ", "cc"])]
    #[case::an_exhausted_budget_splits_at_every_break("a b c", 0, &["a ", "b ", "c"])]
    fn concatenated_parts_fills_each_part_to_the_budget(
        #[case] content: &str,
        #[case] budget: usize,
        #[case] expected: &[&str],
    ) {
        assert_eq!(
            concatenated_parts(content, budget).as_deref(),
            Some(expected)
        );
    }

    #[rstest]
    fn concatenated_parts_refuses_a_literal_without_an_interior_break(
        #[values(
            "",
            "unbreakable",
            "https://example.invalid/a/b",
            "trailing ",
            " leading"
        )]
        content: &str,
    ) {
        assert!(concatenated_parts(content, 4).is_none(), "{content:?}");
    }

    #[rstest]
    fn concatenated_parts_refuses_content_the_budget_already_holds(
        #[values(19, 40, usize::MAX)] budget: usize,
    ) {
        assert!(concatenated_parts("the quick brown fox", budget).is_none());
    }

    #[rstest]
    #[case("the quick brown", &[4, 10])]
    #[case("   opens after the run", &[9, 15, 19])]
    #[case("trailing run  ", &[9])]
    #[case("bound\u{a0}pair", &[7])]
    #[case("solid", &[])]
    fn split_points_marks_the_character_after_each_interior_run(
        #[case] content: &str,
        #[case] expected: &[usize],
    ) {
        assert_eq!(split_points(content).collect::<Vec<_>>(), expected);
    }

    proptest! {
        /// The parts an adjacent-literal form carries rejoin into the
        /// literal they replaced, whatever the budget.
        #[test]
        fn parts_rejoin_into_the_content_they_split(
            content in "[a-z \\\\\"\t\u{a0}]{0,64}",
            budget in 0usize..24,
        ) {
            if let Some(parts) = concatenated_parts(&content, budget) {
                prop_assert!(parts.len() > 1);
                prop_assert_eq!(parts.concat(), content);
            }
        }
    }
}
