//! Normalizes blank-line spacing between adjacent statements at module,
//! class, and function scopes. The walker pairs each statement with its
//! predecessor and emits edits to bring the gap to the canonical count
//! returned by `canonical_blanks`. Own-line comments between adjacent
//! statements carry 1 blank line above the comment block, 0 blank lines
//! below a description block, and 1 blank line below a banner block.

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::{CmpOp, Expr, Stmt};
use ruff_python_trivia::{lines_after, lines_before, BackwardsTokenizer, CommentRanges};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::config::Config;
use crate::primitives::edit::singleton_groups;
use crate::primitives::imports::import_group;
use crate::primitives::scope::BodyScope;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct BlankLines {
    first_party: Vec<String>,
}

impl BlankLines {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            first_party: config.first_party(),
        }
    }
}

impl Rule for BlankLines {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        let mut walker = Walker {
            edits: Vec::new(),
            first_party: &self.first_party,
            source,
        };
        walker.pair_siblings(body, BodyScope::Module);
        walker.visit_body(body);
        singleton_groups(walker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Walker<'a> {
    edits: Vec<Edit>,
    first_party: &'a [String],
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

    /// Places `target_newlines` line breaks between `block_end` and
    /// `curr_line_start`. Emits a replacement edit when the actual
    /// count differs.
    fn normalize_below_block(
        &mut self,
        block_end: TextSize,
        curr_line_start: TextSize,
        target_newlines: u32,
    ) {
        if lines_after(block_end, self.source.text()) == target_newlines {
            return;
        }
        self.edits.push(Edit::range_replacement(
            self.source.newline_str().repeat(target_newlines as usize),
            TextRange::new(block_end, curr_line_start),
        ));
    }

    fn pair_in_scope(&mut self, header: &Stmt, body: &[Stmt], scope: BodyScope) {
        if let Some(first) = body.first() {
            let prev_end = header_signature_end(self.source, first.start());
            self.pair_with_end(header, prev_end, first, scope);
        }
        self.pair_siblings(body, scope);
    }

    fn pair_siblings(&mut self, body: &[Stmt], scope: BodyScope) {
        for (prev, curr) in body.iter().zip(body.iter().skip(1)) {
            self.pair_with_end(prev, prev.end(), curr, scope);
        }
    }

    fn pair_with_end(&mut self, prev: &Stmt, prev_end: TextSize, curr: &Stmt, scope: BodyScope) {
        let Some(canonical) = canonical_blanks(prev, curr, scope, self.first_party) else {
            return;
        };
        let block = leading_block_of(self.source, prev_end, curr);
        let curr_line_start = self.source.text().line_start(curr.start());
        let above_line_start = block.map_or(curr_line_start, TextRange::start);
        self.normalize_above(above_line_start, canonical + 1);
        if let Some(b) = block {
            let below_target = 1 + u32::from(is_banner_block(self.source, b));
            self.normalize_below_block(b.end(), curr_line_start, below_target);
        }
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(c) => self.pair_in_scope(stmt, &c.body, BodyScope::Class),
            Stmt::FunctionDef(f) => self.pair_in_scope(stmt, &f.body, BodyScope::Function),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns the canonical blank-line count for the pair `(prev, curr)`
/// at `scope`. `None` means no case applies and the pair is skipped,
/// leaving any in-gap whitespace and comments untouched. For `Class`
/// and `Function` scopes, the pair includes the scope-entry transition
/// wherein `prev` is the enclosing `ClassDef` or `FunctionDef` itself
/// and `curr` is the first body member.
fn canonical_blanks(
    prev: &Stmt,
    curr: &Stmt,
    scope: BodyScope,
    first_party: &[String],
) -> Option<u32> {
    match scope {
        BodyScope::Class => class_scope_blanks(prev, curr),
        BodyScope::Function => function_scope_blanks(prev, curr),
        BodyScope::Module => module_scope_blanks(prev, curr, first_party),
    }
}

/// Class-scope pair dispatch. The class header pairs with its first
/// body member, with 0 blank lines before a docstring and 1 otherwise.
/// Class-field → method and method-after-method pairs carry 1. Any
/// docstring-predecessor pair carries 1.
fn class_scope_blanks(prev: &Stmt, curr: &Stmt) -> Option<u32> {
    match (prev, curr) {
        (Stmt::ClassDef(_), _) => Some(u32::from(!is_docstring_stmt(curr))),
        (Stmt::FunctionDef(_) | Stmt::AnnAssign(_) | Stmt::Assign(_), Stmt::FunctionDef(_)) => {
            Some(1)
        }
        _ if is_docstring_stmt(prev) => Some(1),
        _ => None,
    }
}

/// Function-scope pair dispatch. The function header carries 1 blank
/// line before its first body statement when that statement is a
/// compound-body opener.
fn function_scope_blanks(prev: &Stmt, curr: &Stmt) -> Option<u32> {
    match (prev, curr) {
        (
            Stmt::FunctionDef(_),
            Stmt::For(_)
            | Stmt::If(_)
            | Stmt::Match(_)
            | Stmt::Try(_)
            | Stmt::While(_)
            | Stmt::With(_),
        ) => Some(1),
        _ => None,
    }
}

/// Returns the position immediately after the `:` that introduces a
/// class or function body whose first statement starts at `body_start`.
/// Scans backward from `body_start` through whitespace and comments,
/// landing on the first non-trivia token. Falls back to `body_start`
/// when the scan finds none.
fn header_signature_end(source: &Source, body_start: TextSize) -> TextSize {
    BackwardsTokenizer::up_to(body_start, source.text(), source.comment_ranges())
        .skip_trivia()
        .next()
        .map_or(body_start, |t| t.end())
}

/// True when any line in the comment block is a decorative rule line.
fn is_banner_block(source: &Source, block: TextRange) -> bool {
    source.slice(block).lines().any(is_rule_line)
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
    left.id == "__name__" && right.value.to_str() == "__main__"
}

/// True when `line` is a comment whose body, after stripping the
/// leading `#` and surrounding whitespace, consists of 5 or more
/// identical non-alphanumeric characters.
fn is_rule_line(line: &str) -> bool {
    let stripped = line
        .trim_start()
        .strip_prefix('#')
        .map(str::trim)
        .unwrap_or("");
    let bytes = stripped.as_bytes();
    bytes.len() >= 5 && !bytes[0].is_ascii_alphanumeric() && bytes.iter().all(|&b| b == bytes[0])
}

/// Returns the contiguous range of own-line comments lying between
/// `prev_end` and `curr.start()`. `None` when no own-line comment
/// sits in that gap. End-of-line comments on the predecessor's line
/// are excluded.
fn leading_block_of(source: &Source, prev_end: TextSize, curr: &Stmt) -> Option<TextRange> {
    let text = source.text();
    let mut own_lines = source
        .comment_ranges()
        .comments_in_range(TextRange::new(prev_end, curr.start()))
        .iter()
        .copied()
        .filter(|r| CommentRanges::is_own_line(r.start(), text));
    let first = own_lines.next()?;
    let last = own_lines.next_back().unwrap_or(first);
    Some(TextRange::new(text.line_start(first.start()), last.end()))
}

/// Module-scope pair dispatch. A statement following an
/// `if __name__ == "__main__":` block carries 1 blank line. A pair of
/// import statements lands 1 blank line when they fall in different
/// canonical groups (bare, external `from`, local-package) and none
/// when they share a group. A top-level `FunctionDef` or `ClassDef`
/// carries 2 blank lines before it. An `Assign` or `AnnAssign`
/// following a top-level `FunctionDef` or `ClassDef` carries 2.
fn module_scope_blanks(prev: &Stmt, curr: &Stmt, first_party: &[String]) -> Option<u32> {
    if is_main_guard(prev) {
        return Some(1);
    }
    if let Some((prev_group, curr_group)) =
        import_group(prev, first_party).zip(import_group(curr, first_party))
    {
        return (prev_group != curr_group).then_some(1);
    }
    match (prev, curr) {
        (_, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) => Some(2),
        (Stmt::FunctionDef(_) | Stmt::ClassDef(_), Stmt::Assign(_) | Stmt::AnnAssign(_)) => Some(2),
        _ => None,
    }
}

/// Returns the start of the contiguous ASCII-whitespace run immediately
/// preceding `offset` in `text`.
fn whitespace_start_before(text: &str, offset: TextSize) -> TextSize {
    let trimmed = text[..offset.to_usize()].trim_end_matches(|c: char| c.is_ascii_whitespace());
    TextSize::of(trimmed)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

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
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_class_field_to_method_returns_one(
        #[values(
            "class C:\n    x: int = 1\n    def m(self): pass\n",
            "class C:\n    x = 1\n    def m(self): pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_class_header_to_docstring_returns_zero() {
        let s = parse("class C:\n    '''doc'''\n    pass\n");
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &class.body[0], BodyScope::Class, &[]),
            Some(0),
        );
    }

    #[rstest]
    fn canonical_blanks_class_header_to_first_member_returns_one(
        #[values(
            "class C:\n    def m(self): pass\n",
            "class C:\n    @decorator\n    def m(self): pass\n",
            "class C:\n    x: int = 1\n",
            "class C:\n    x = 1\n",
            "class C:\n    class Inner:\n        pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &class.body[0], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_function_header_to_compound_body_returns_one(
        #[values(
            "def f():\n    for x in y:\n        pass\n",
            "def f():\n    if x:\n        pass\n",
            "def f():\n    match x:\n        case _: pass\n",
            "def f():\n    try:\n        pass\n    except Exception:\n        pass\n",
            "def f():\n    while x:\n        pass\n",
            "def f():\n    with x:\n        pass\n",
            "async def f():\n    async for x in y:\n        pass\n",
            "async def f():\n    async with x:\n        pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let func = s.ast().body[0].as_function_def_stmt().expect("def");
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &func.body[0], BodyScope::Function, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_function_header_to_simple_stmt_returns_none(
        #[values(
            "def f():\n    x = 1\n",
            "def f():\n    return None\n",
            "def f():\n    '''doc'''\n",
            "def f():\n    def inner(): pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let func = s.ast().body[0].as_function_def_stmt().expect("def");
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &func.body[0], BodyScope::Function, &[]),
            None,
        );
    }

    #[test]
    fn canonical_blanks_in_class_body_pairs_method_after_method_to_one() {
        let s = parse("class C:\n    def m1(self): pass\n    def m2(self): pass\n");
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_in_function_body_returns_none() {
        let s = parse("def f():\n    x = 1\n    y = 2\n");
        let func = s.ast().body[0].as_function_def_stmt().expect("def");
        assert_eq!(
            canonical_blanks(&func.body[0], &func.body[1], BodyScope::Function, &[]),
            None,
        );
    }

    #[test]
    fn canonical_blanks_module_after_main_guard_returns_one() {
        let s = parse(&format!("{}xs = 1\n", main_guard_src()));
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(1)
        );
    }

    #[rstest]
    fn canonical_blanks_module_assignment_after_def_or_class_returns_two(
        #[values(
            "class C: pass\nPORT = 8080\n",
            "class C: pass\nPORT: int = 8080\n",
            "def f(): pass\nPORT = 8080\n",
            "def f(): pass\nPORT: int = 8080\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(2),
        );
    }

    #[test]
    fn canonical_blanks_module_class_after_module_stmt_returns_two() {
        let s = parse("x = 1\nclass C: pass\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(2)
        );
    }

    #[test]
    fn canonical_blanks_module_def_after_module_stmt_returns_two() {
        let s = parse("x = 1\ndef f(): pass\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(2)
        );
    }

    #[rstest]
    #[case("from os import path\nfrom . import x\n", &[], Some(1))]
    #[case("from os import path\nfrom myapp import x\n", &["myapp"], Some(1))]
    #[case("import os\nimport myapp\n", &["myapp"], Some(1))]
    #[case("import myapp\nfrom myapp import x\n", &["myapp"], None)]
    #[case("from myapp import a\nfrom myapp.db import b\n", &["myapp"], None)]
    fn canonical_blanks_module_import_group_boundary_separates_distinct_groups(
        #[case] src: &str,
        #[case] first_party: &[&str],
        #[case] expected: Option<u32>,
    ) {
        let list: Vec<String> = first_party.iter().map(|&s| s.to_owned()).collect();
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &list),
            expected,
        );
    }

    #[rstest]
    fn canonical_blanks_module_import_kind_boundary_returns_one(
        #[values(
            "import os\nfrom sys import argv\n",
            "from sys import argv\nimport os\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_module_same_kind_import_run_returns_none(
        #[values(
            "import os\nimport sys\n",
            "from os import path\nfrom sys import argv\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            None
        );
    }

    #[test]
    fn canonical_blanks_unrelated_pair_returns_none() {
        let s = parse("x = 1\ny = 2\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            None
        );
    }

    #[test]
    fn header_signature_end_handles_multi_line_function_signature() {
        let s = parse("def f(\n    x,\n    y,\n):\n    pass\n");
        let func = s.ast().body[0].as_function_def_stmt().expect("def");
        let end = header_signature_end(&s, func.body[0].start());
        assert!(s.text()[..end.to_usize()].ends_with("):"));
    }

    #[test]
    fn header_signature_end_points_after_colon_in_simple_class() {
        let s = parse("class C:\n    pass\n");
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        let end = header_signature_end(&s, class.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "class C:");
    }

    #[test]
    fn header_signature_end_points_after_colon_in_simple_function() {
        let s = parse("def f():\n    pass\n");
        let func = s.ast().body[0].as_function_def_stmt().expect("def");
        let end = header_signature_end(&s, func.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "def f():");
    }

    #[test]
    fn header_signature_end_skips_eol_comment_on_header_line() {
        let s = parse("class C:  # eol\n    pass\n");
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        let end = header_signature_end(&s, class.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "class C:");
    }

    #[test]
    fn header_signature_end_skips_own_line_comment_above_body() {
        let s = parse("class C:\n    # comment\n    pass\n");
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        let end = header_signature_end(&s, class.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "class C:");
    }

    #[test]
    fn is_banner_block_detects_block_with_any_rule_line() {
        let s = parse(
            "x = 1\n# ========================\n# Section: helpers\n# ========================\ndef f(): pass\n",
        );
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
        assert!(is_banner_block(&s, block));
    }

    #[test]
    fn is_banner_block_returns_false_for_all_prose_block() {
        let s = parse("x = 1\n# describes f\n# helper\ndef f(): pass\n");
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
        assert!(!is_banner_block(&s, block));
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

    #[rstest]
    fn is_main_guard_rejects_other_if_conditions(
        #[values(
            "if x:\n    pass\n",
            "if __name__ != \"__main__\":\n    pass\n",
            "if __name__ == \"main\":\n    pass\n",
            "if other == \"__main__\":\n    pass\n",
            "if __name__ == __main__:\n    pass\n",
            "if __name__ == \"__main__\" and x:\n    pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        assert!(!is_main_guard(&s.ast().body[0]));
    }

    #[rstest]
    fn is_rule_line_accepts_canonical_decorative_runs(
        #[values("# =====", "# -----", "# *****", "# _____", "# ~~~~~", "##########")] line: &str,
    ) {
        assert!(is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_rejects_alpha_prose(
        #[values("# describes f", "# Section: helpers", "# x")] line: &str,
    ) {
        assert!(!is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_rejects_mixed_characters(
        #[values("# = = = =", "# -=-=-=", "# - - -")] line: &str,
    ) {
        assert!(!is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_rejects_short_runs(#[values("# ====", "# ---", "# ", "#")] line: &str) {
        assert!(!is_rule_line(line));
    }

    #[test]
    fn leading_block_of_returns_block_for_chain_of_own_line_comments() {
        let s = parse("x = 1\n# a\n# b\ndef f(): pass\n");
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
        let comments = s.comment_ranges();
        assert_eq!(block.start(), s.text().line_start(comments[0].start()));
        assert_eq!(block.end(), comments[1].end());
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
