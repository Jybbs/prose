//! Reports each table build, each carry across a reparse, and each rule
//! a settle check re-applies to stderr while `PROSE_CARRY_TRACE` is
//! set, one tab-separated line per event.

use std::{
    env,
    fmt::{self, Display},
    sync::LazyLock,
};

use crate::rule::RuleId;

/// True where `PROSE_CARRY_TRACE` is set in the environment.
static ENABLED: LazyLock<bool> = LazyLock::new(|| env::var_os("PROSE_CARRY_TRACE").is_some());

/// What became of one table at a reparse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Outcome {
    /// The rule preserves the table, which the source before it never
    /// built.
    Absent,
    /// The table moved into the reparsed source.
    Carried,
    /// The rule declares its edits leave the table nowhere to move.
    Declined,
    /// An edit replaced an offset the table names, so the move failed.
    Dropped,
}

impl Outcome {
    /// The outcome for a table `permitted` to survive the rule, `held`
    /// by the source before it, and `moved` into the source after it.
    pub(super) fn of(permitted: bool, held: bool, moved: bool) -> Self {
        match (permitted, held, moved) {
            (false, ..) => Self::Declined,
            (true, false, _) => Self::Absent,
            (true, true, false) => Self::Dropped,
            (true, true, true) => Self::Carried,
        }
    }
}

impl Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Absent => "absent",
            Self::Carried => "carried",
            Self::Declined => "declined",
            Self::Dropped => "dropped",
        })
    }
}

/// Reports that a walk built `table`.
pub(super) fn built(table: &'static str) {
    if *ENABLED {
        eprintln!("build\t{table}");
    }
}

/// Reports what became of `table` at the reparse past `rule`.
pub(super) fn carried(rule: RuleId, table: &'static str, outcome: Outcome) {
    if *ENABLED {
        eprintln!("carry\t{rule}\t{table}\t{outcome}");
    }
}

/// Reports that a settle check under `pass` re-applied `rule`.
pub(crate) fn reapplied(pass: &'static str, rule: RuleId) {
    if *ENABLED {
        eprintln!("second\t{pass}\t{rule}");
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::declined(false, true, true, Outcome::Declined)]
    #[case::absent(true, false, false, Outcome::Absent)]
    #[case::dropped(true, true, false, Outcome::Dropped)]
    #[case::carried(true, true, true, Outcome::Carried)]
    fn outcome_of_names_what_became_of_the_table(
        #[case] permitted: bool,
        #[case] held: bool,
        #[case] moved: bool,
        #[case] expected: Outcome,
    ) {
        assert_eq!(Outcome::of(permitted, held, moved), expected);
        assert_eq!(expected.to_string(), format!("{expected:?}").to_lowercase());
    }
}
