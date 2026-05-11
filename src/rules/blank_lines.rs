//! Normalizes vertical spacing between adjacent statements. Module-level
//! `def` and `class` carry 2 blank lines before them. Methods inside a
//! class body carry 1 blank line. A class-scope predecessor that is a
//! string-literal docstring carries 1 blank line before the next member.
//! A module-level statement following an `if __name__ == "__main__":`
//! block carries 1 blank line of separation. Own-line comments between
//! adjacent statements carry 1 blank line of separation from the
//! following statement.

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::{CmpOp, Expr, Stmt};
use ruff_python_trivia::{lines_after, lines_before, CommentRanges};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::config::Config;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct BlankLines;

impl BlankLines {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for BlankLines {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let body = &source.ast().body;
        let mut walker = Walker {
            edits: Vec::new(),
            source,
        };
        walker.pair_count(body, BodyScope::Module);
        walker.visit_body(body);
        walker.edits
    }

    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(BlankLines))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyScope {
    Class,
    Function,
    Module,
}

#[derive(Clone, Copy)]
struct CommentBlock {
    bottom_end: TextSize,
    top_line_start: TextSize,
}

struct Walker<'a> {
    edits: Vec<Edit>,
    source: &'a Source,
}

impl Walker<'_> {
    /// Places `target_newlines` line breaks immediately above
    /// `line_start`. Emits a replacement edit when the actual count
    /// differs. Preserves any indent that sits on `line_start`'s line.
    fn normalize_above(&mut self, line_start: TextSize, target_newlines: u32) {
        let text = self.source.text();
        if lines_before(line_start, text) == target_newlines {
            return;
        }
        let span_start = whitespace_start_before(text, line_start);
        let replacement = self.source.newline_str().repeat(target_newlines as usize);
        self.edits.push(Edit::range_replacement(
            replacement,
            TextRange::new(span_start, line_start),
        ));
    }

    /// Places exactly 1 blank line between `block_end` and
    /// `curr_line_start`. The target gap is 2 line breaks: one to end
    /// the block's last line and one to form the blank.
    fn normalize_below_block(&mut self, block_end: TextSize, curr_line_start: TextSize) {
        let target_newlines: u32 = 2;
        if lines_after(block_end, self.source.text()) == target_newlines {
            return;
        }
        let replacement = self.source.newline_str().repeat(target_newlines as usize);
        self.edits.push(Edit::range_replacement(
            replacement,
            TextRange::new(block_end, curr_line_start),
        ));
    }

    fn pair_count(&mut self, body: &[Stmt], scope: BodyScope) {
        for (prev, curr) in body.iter().zip(body.iter().skip(1)) {
            let Some(canonical) = canonical_blanks(prev, curr, scope) else {
                continue;
            };
            let block = leading_block_of(self.source, prev.end(), curr);
            self.push_pair_edits(curr, canonical, block);
        }
    }

    fn push_pair_edits(&mut self, curr: &Stmt, canonical: u32, block: Option<CommentBlock>) {
        let curr_line_start = self.source.text().line_start(curr.start());
        let above_line_start = block.map_or(curr_line_start, |b| b.top_line_start);
        self.normalize_above(above_line_start, canonical + 1);
        if let Some(b) = block {
            self.normalize_below_block(b.bottom_end, curr_line_start);
        }
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(c) => self.pair_count(&c.body, BodyScope::Class),
            Stmt::FunctionDef(f) => self.pair_count(&f.body, BodyScope::Function),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns the canonical blank-line count for the pair `(prev, curr)`
/// at `scope`. `None` means no case applies; the above-block gap is
/// left alone, but a leading comment block still triggers below-gap
/// normalization.
fn canonical_blanks(prev: &Stmt, curr: &Stmt, scope: BodyScope) -> Option<u32> {
    match scope {
        BodyScope::Class => (is_docstring_stmt(prev)
            || matches!((prev, curr), (Stmt::FunctionDef(_), Stmt::FunctionDef(_))))
        .then_some(1),
        BodyScope::Function => None,
        BodyScope::Module => {
            if is_main_guard(prev) {
                Some(1)
            } else {
                matches!(curr, Stmt::FunctionDef(_) | Stmt::ClassDef(_)).then_some(2)
            }
        }
    }
}

/// True when `stmt` is `if __name__ == "__main__":`.
fn is_main_guard(stmt: &Stmt) -> bool {
    let Some(if_stmt) = stmt.as_if_stmt() else {
        return false;
    };
    let Some(cmp) = if_stmt.test.as_compare_expr() else {
        return false;
    };
    let ([CmpOp::Eq], Some(left), Some(right)) = (
        cmp.ops.as_ref(),
        cmp.left.as_name_expr(),
        cmp.comparators
            .first()
            .and_then(Expr::as_string_literal_expr),
    ) else {
        return false;
    };
    left.id == "__name__" && right.value == *"__main__"
}

/// Returns the contiguous range of own-line comments lying between
/// `prev_end` and `curr.start()`. `None` when no own-line comment
/// sits in that gap. End-of-line comments on the predecessor's line
/// are excluded.
fn leading_block_of(source: &Source, prev_end: TextSize, curr: &Stmt) -> Option<CommentBlock> {
    let text = source.text();
    let mut own_lines = source
        .comment_ranges()
        .comments_in_range(TextRange::new(prev_end, curr.start()))
        .iter()
        .copied()
        .filter(|r| CommentRanges::is_own_line(r.start(), text));
    let first = own_lines.next()?;
    let last = own_lines.next_back().unwrap_or(first);
    Some(CommentBlock {
        top_line_start: text.line_start(first.start()),
        bottom_end: last.end(),
    })
}

/// Returns the start of the contiguous ASCII-whitespace run immediately
/// preceding `offset` in `text`.
fn whitespace_start_before(text: &str, offset: TextSize) -> TextSize {
    let trimmed = text[..offset.to_usize()].trim_end_matches(|c: char| c.is_ascii_whitespace());
    TextSize::of(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    fn main_guard_src() -> &'static str {
        "if __name__ == \"__main__\":\n    main()\n"
    }

    #[test]
    fn canonical_blanks_class_docstring_predecessor_returns_one() {
        let s = parse("class C:\n    '''doc'''\n    def m1(self): pass\n");
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_in_class_body_pairs_method_after_method_to_one() {
        let s = parse("class C:\n    def m1(self): pass\n    def m2(self): pass\n");
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_in_function_body_returns_none() {
        let s = parse("def f():\n    x = 1\n    y = 2\n");
        let func = s.ast().body[0].as_function_def_stmt().expect("def");
        assert_eq!(
            canonical_blanks(&func.body[0], &func.body[1], BodyScope::Function),
            None,
        );
    }

    #[test]
    fn canonical_blanks_module_after_main_guard_returns_one() {
        let s = parse(&format!("{}xs = 1\n", main_guard_src()));
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module),
            Some(1)
        );
    }

    #[test]
    fn canonical_blanks_module_def_after_module_stmt_returns_two() {
        let s = parse("x = 1\ndef f(): pass\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module),
            Some(2)
        );
    }

    #[test]
    fn canonical_blanks_unrelated_pair_returns_none() {
        let s = parse("x = 1\ny = 2\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module),
            None
        );
    }

    #[test]
    fn is_main_guard_accepts_canonical_form() {
        let s = parse(main_guard_src());
        assert!(is_main_guard(&s.ast().body[0]));
    }

    #[test]
    fn is_main_guard_rejects_non_if_statements() {
        let s = parse("x = 1\n");
        assert!(!is_main_guard(&s.ast().body[0]));
    }

    #[test]
    fn is_main_guard_rejects_other_if_conditions() {
        for src in [
            "if x:\n    pass\n",
            "if __name__ != \"__main__\":\n    pass\n",
            "if __name__ == \"main\":\n    pass\n",
            "if other == \"__main__\":\n    pass\n",
            "if __name__ == __main__:\n    pass\n",
            "if __name__ == \"__main__\" and x:\n    pass\n",
        ] {
            let s = parse(src);
            assert!(!is_main_guard(&s.ast().body[0]), "src = {src:?}");
        }
    }

    #[test]
    fn leading_block_of_returns_block_for_chain_of_own_line_comments() {
        let s = parse("x = 1\n# a\n# b\ndef f(): pass\n");
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
        let comments: Vec<TextRange> = s.comment_ranges().iter().copied().collect();
        assert_eq!(
            block.top_line_start,
            s.text().line_start(comments[0].start())
        );
        assert_eq!(block.bottom_end, comments[1].end());
    }

    #[test]
    fn leading_block_of_returns_none_when_no_own_line_comments_between() {
        let s = parse("x = 1\ndef f(): pass\n");
        let body = &s.ast().body;
        assert!(leading_block_of(&s, body[0].end(), &body[1]).is_none());
    }

    #[test]
    fn leading_block_of_skips_trailing_end_of_line_comments() {
        let s = parse("x = 1  # trail\ndef f(): pass\n");
        let body = &s.ast().body;
        assert!(leading_block_of(&s, body[0].end(), &body[1]).is_none());
    }

    #[test]
    fn whitespace_start_before_handles_crlf() {
        assert_eq!(
            whitespace_start_before("a\r\n\r\nb", TextSize::new(5)),
            TextSize::new(1),
        );
    }

    #[test]
    fn whitespace_start_before_returns_zero_for_leading_whitespace() {
        assert_eq!(
            whitespace_start_before("   \n\n\nx", TextSize::new(6)),
            TextSize::new(0),
        );
    }

    #[test]
    fn whitespace_start_before_stops_at_non_whitespace() {
        assert_eq!(
            whitespace_start_before("ab\n\ncd", TextSize::new(4)),
            TextSize::new(2),
        );
    }
}
