//! Flags own-line comments shaped as numbered procedural narration
//! (`# 1. text`, `# Step 2: text`, `# step 3. text`). Pragmas and
//! decimal-version comments are excluded.

use ruff_diagnostics::Edit;
use ruff_python_trivia::{is_pragma_comment, CommentRanges};

use crate::config::Config;
use crate::diagnostics::{Diagnostic, Severity};
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct NoStepNarration;

impl NoStepNarration {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for NoStepNarration {
    fn apply(&self, _source: &Source) -> Vec<Edit> {
        Vec::new()
    }

    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(NoStepNarration))
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let text = source.text();
        let rule = self.id();
        let message = self.message();
        source
            .comment_ranges()
            .into_iter()
            .filter(|range| CommentRanges::is_own_line(range.start(), text))
            .filter(|range| is_step_narration(&text[*range]))
            .map(|range| Diagnostic {
                fix: None,
                message: message.to_owned(),
                range,
                rule,
                severity: Severity::Lint,
            })
            .collect()
    }
}

/// `\s+\S` against `rest`, accepting only spaces and tabs.
fn has_space_then_text(rest: &str) -> bool {
    let trimmed = rest.trim_start_matches(is_space_or_tab);
    trimmed.len() < rest.len() && !trimmed.is_empty()
}

fn is_space_or_tab(c: char) -> bool {
    matches!(c, ' ' | '\t')
}

/// Returns `true` when `comment` matches the numbered-step shape and
/// is not a pragma or decimal-version comment.
fn is_step_narration(comment: &str) -> bool {
    if is_pragma_comment(comment) {
        return false;
    }
    let Some(body) = comment.strip_prefix('#') else {
        return false;
    };
    let body = body.trim_start_matches(is_space_or_tab);
    matches_step_word(body) || matches_numeric_dot(body)
}

/// Matches the `^\d+\.\s+\S` body, rejecting decimal versions where a
/// digit follows the dot.
fn matches_numeric_dot(body: &str) -> bool {
    let digits = body.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return false;
    }
    let Some(after_dot) = body[digits..].strip_prefix('.') else {
        return false;
    };
    if after_dot.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    has_space_then_text(after_dot)
}

/// Matches the `^[Ss]tep\s+\d+[:.]\s+\S` body.
fn matches_step_word(body: &str) -> bool {
    let Some(after_step) = body
        .strip_prefix("Step")
        .or_else(|| body.strip_prefix("step"))
    else {
        return false;
    };
    if !after_step.starts_with(is_space_or_tab) {
        return false;
    }
    let rest = after_step.trim_start_matches(is_space_or_tab);
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return false;
    }
    let Some(after_sep) = rest[digits..].strip_prefix([':', '.']) else {
        return false;
    };
    has_space_then_text(after_sep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    #[test]
    fn apply_never_produces_edits() {
        let source = parse("# 1. step\nx = 1\n");
        assert!(NoStepNarration.apply(&source).is_empty());
    }

    #[test]
    fn matches_numeric_dot_accepts_no_space_after_hash_and_multi_digit() {
        assert!(is_step_narration("#1. open file"));
        assert!(is_step_narration("#  12. parse header"));
    }

    #[test]
    fn matches_numeric_dot_requires_whitespace_after_dot() {
        assert!(!is_step_narration("# 1.open"));
        assert!(!is_step_narration("# 1."));
    }

    #[test]
    fn matches_step_word_accepts_lowercase_leader() {
        assert!(is_step_narration("# step 2: parse"));
        assert!(is_step_narration("# step 2. parse"));
    }

    #[test]
    fn matches_step_word_rejects_uppercase_or_partial_word() {
        assert!(!is_step_narration("# STEP 1: validate"));
        assert!(!is_step_narration("# stepping 1: validate"));
        assert!(!is_step_narration("# StEp 1: validate"));
    }

    #[test]
    fn matches_step_word_requires_separator_and_text() {
        assert!(!is_step_narration("# Step 1 validate"));
        assert!(!is_step_narration("# Step 1:"));
        assert!(!is_step_narration("# Step abc: validate"));
    }
}
