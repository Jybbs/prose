//! Helpers shared across `#[cfg(test)] mod tests` blocks.

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

use crate::{
    diagnostics::Diagnostic,
    rule::{Rule, RuleId},
    source::Source,
};

/// Test-only rule that returns the fix groups supplied at
/// construction.
pub(crate) struct GroupSentinelRule {
    pub(crate) groups: Vec<Vec<Edit>>,
    pub(crate) id: RuleId,
}

impl Rule for GroupSentinelRule {
    fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
        self.groups.clone()
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn message(&self) -> &'static str {
        "group test rule"
    }
}

pub(crate) fn assert_send_sync<T: Send + Sync>() {}

/// Returns a rule whose single edit rewrites the leading statement
/// into unparseable source.
pub(crate) fn breaks_parse() -> GroupSentinelRule {
    GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement(
            "def foo(".to_owned(),
            range(0, 5),
        )]],
        id: RuleId::from("breaks-parse"),
    }
}

/// Format diagnostic with a safe single-edit fix.
pub(crate) fn format_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::format(
        RuleId::from("rewrite-x"),
        vec![Edit::range_replacement("y".to_owned(), range)],
        "rewrite x to y".to_owned(),
    )
}

pub(crate) fn parse(src: &str) -> Source {
    src.parse().expect("test source parses")
}

pub(crate) fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(start.into(), end.into())
}
