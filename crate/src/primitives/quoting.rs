//! Shared guards on the quote characters a Python literal carries.

/// The canonical triple-quoted delimiter, the frame `docstring-frame`
/// re-delimits every docstring to.
pub(crate) const TRIPLE_QUOTE: &str = "\"\"\"";

/// True when re-delimiting `parts` to the `"""` frame would break the
/// literal. A `"""` run collides with the closer wherever it sits, and
/// a trailing `"` collides only where `closer_abuts`, which is false
/// for a rewrite that lands the closer on its own line.
pub(crate) fn abuts_triple_closer(parts: &[&str], closer_abuts: bool) -> bool {
    parts.iter().any(|part| part.contains(TRIPLE_QUOTE))
        || (closer_abuts && parts.last().is_some_and(|part| part.ends_with('"')))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn abuts_triple_closer_reads_a_run_in_any_part() {
        let parts = &["opens \"\"\" here", "and ends plain"];
        assert!(abuts_triple_closer(parts, true));
        assert!(
            abuts_triple_closer(parts, false),
            "a run collides wherever it sits"
        );
    }

    #[test]
    fn abuts_triple_closer_reads_a_tail_on_the_last_part_alone() {
        assert!(!abuts_triple_closer(
            &["ends in a quote\"", "and then plain"],
            true
        ));
        assert!(abuts_triple_closer(
            &["opens plain", "and ends in a quote\""],
            true
        ));
    }

    #[rstest]
    #[case(&["plain"], true, false)]
    #[case(&["holds a \"\"\" run"], true, true)]
    #[case(&["holds a \"\"\" run"], false, true)]
    #[case(&["ends in a quote\""], true, true)]
    #[case(&["ends in a quote\""], false, false)]
    #[case(&["a \" b"], true, false)]
    #[case(&[], true, false)]
    fn abuts_triple_closer_reads_the_run_always_and_the_tail_only_when_it_meets_the_closer(
        #[case] parts: &[&str],
        #[case] closer_abuts: bool,
        #[case] expected: bool,
    ) {
        assert_eq!(abuts_triple_closer(parts, closer_abuts), expected);
    }
}
