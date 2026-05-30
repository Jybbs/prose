//! Flags own-line comments shaped as numbered procedural narration
//! (`# 1. text`, `# Step 2: text`, `# step 3. text`). Pragmas and
//! decimal-version comments are excluded.

use ruff_python_trivia::{
    CommentRanges, Cursor, PythonWhitespace, is_pragma_comment, is_python_whitespace,
};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct NoStepNarration;

impl NoStepNarration {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for NoStepNarration {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let text = source.text();
        let rule = self.id();
        let message = self.message();
        source
            .comment_ranges()
            .into_iter()
            .filter(|range| CommentRanges::is_own_line(range.start(), text))
            .filter(|&range| is_step_narration(&text[range]))
            .map(|range| Diagnostic::lint(rule, range, message.to_owned()))
            .collect()
    }
}

/// Returns `true` when `comment` matches the numbered-step shape and
/// is not a pragma comment.
fn is_step_narration(comment: &str) -> bool {
    if is_pragma_comment(comment) {
        return false;
    }
    let Some(body) = comment.strip_prefix('#') else {
        return false;
    };
    let body = body.trim_whitespace_start();
    matches_step_word(body) || matches_numeric_dot(body)
}

/// Matches the `^\d+\.\s+\S` body.
fn matches_numeric_dot(body: &str) -> bool {
    let mut cursor = Cursor::new(body);
    if !cursor.eat_if(|c| c.is_ascii_digit()) {
        return false;
    }
    cursor.eat_while(|c| c.is_ascii_digit());
    if !cursor.eat_char('.') {
        return false;
    }
    if !cursor.eat_if(is_python_whitespace) {
        return false;
    }
    cursor.eat_while(is_python_whitespace);
    !cursor.is_eof()
}

/// Matches the `^[Ss]tep\s+\d+[:.]\s+\S` body.
fn matches_step_word(body: &str) -> bool {
    let Some(rest) = body
        .strip_prefix("Step")
        .or_else(|| body.strip_prefix("step"))
    else {
        return false;
    };
    let mut cursor = Cursor::new(rest);
    if !cursor.eat_if(is_python_whitespace) {
        return false;
    }
    cursor.eat_while(is_python_whitespace);
    if !cursor.eat_if(|c| c.is_ascii_digit()) {
        return false;
    }
    cursor.eat_while(|c| c.is_ascii_digit());
    if !cursor.eat_if(|c| c == ':' || c == '.') {
        return false;
    }
    if !cursor.eat_if(is_python_whitespace) {
        return false;
    }
    cursor.eat_while(is_python_whitespace);
    !cursor.is_eof()
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
