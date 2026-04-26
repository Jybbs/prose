//! Aligns `:` vertically in dict/mapping literals, Pydantic-style
//! class fields, annotated function parameters, and Google/numpy
//! docstring `Args:` sections. Single-line groups and single-item
//! groups pass through, leaving the latter to `singleton_rule`
//! downstream. Each aligned `:` keeps a one-space buffer before the
//! colon.

use std::cmp::Ordering;

use ruff_diagnostics::Edit;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{walk_expr, walk_parameters, walk_stmt, Visitor as AstVisitor};
use ruff_python_ast::{AnyParameterRef, Expr, ExprDict, Parameters, Stmt};
use ruff_python_trivia::{CommentRanges, PythonWhitespace};
use ruff_source_file::{LineRanges, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::aligner;
use crate::source::Source;

pub struct AlignColons {
    settings: aligner::Settings,
}

impl AlignColons {
    pub fn from_config(config: &Config) -> Self {
        Self {
            settings: (&config.rules.align_colons).into(),
        }
    }
}

impl Rule for AlignColons {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            comment_ranges: CommentRanges::from(source.tokens()),
            edits: Vec::new(),
            settings: self.settings,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }

    fn name(&self) -> &'static str {
        "align-colons"
    }
}

struct Visitor<'a> {
    comment_ranges: CommentRanges,
    edits: Vec<Edit>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl Visitor<'_> {
    fn class_field_member(&self, stmt: &Stmt) -> Option<aligner::Member> {
        let ann = stmt.as_ann_assign_stmt()?;
        aligner::line_anchored_member_at_kind(
            self.source,
            TextRange::new(ann.target.end(), ann.annotation.start()),
            TokenKind::Colon,
        )
    }

    /// Returns `true` when every member's colon sits on a distinct
    /// source line. The `line_start`-anchored width math requires this
    /// so two members do not share a prefix region. The colon's
    /// position is recoverable as `gap.end()` because `lhs_info`
    /// constructs the gap with the colon as its end.
    fn distinct_lines(&self, members: &[aligner::Member]) -> bool {
        let text = self.source.text();
        members
            .windows(2)
            .all(|w| text.line_start(w[0].gap.end()) != text.line_start(w[1].gap.end()))
    }

    /// Walks the source lines spanned by a docstring's text range,
    /// locates the first `Args:` header, and returns one alignment
    /// member per entry line in the subsequent indented block. An
    /// entry is any line whose first non-whitespace content runs up
    /// to a `:` before the line ends. Continuation lines, blank
    /// lines, and the next section header end the block.
    fn docstring_args_members(&self, ds_range: TextRange) -> Vec<aligner::Member> {
        let body = self.source.slice(ds_range);
        let Some(header_offset) = find_args_header(body) else {
            return Vec::new();
        };
        let header_indent_len = body[..header_offset]
            .rsplit_once('\n')
            .map_or(header_offset, |(_, last)| last.len());

        let mut members = Vec::new();
        let mut entry_indent_len: Option<usize> = None;
        let after_header = header_offset + "Args:".len();
        for line in body[after_header..].universal_newlines().skip(1) {
            let content = line.as_str();
            let stripped = content.trim_whitespace_start();
            let line_indent_len = content.len() - stripped.len();

            if stripped.is_empty() || line_indent_len <= header_indent_len {
                break;
            }

            match entry_indent_len {
                None => entry_indent_len = Some(line_indent_len),
                Some(expected) => match line_indent_len.cmp(&expected) {
                    Ordering::Greater => continue,
                    Ordering::Less => break,
                    Ordering::Equal => {}
                },
            }

            if let Some(colon_rel) = find_entry_colon(stripped) {
                let line_offset = TextSize::try_from(after_header + line_indent_len + colon_rel)
                    .expect("docstring colon offset fits in TextSize");
                let colon_start = ds_range.start() + line.start() + line_offset;
                members.push(aligner::line_anchored_member(self.source, colon_start));
            }
        }
        members
    }

    /// Emits alignment edits for a qualifying group of members.
    fn emit(&mut self, members: &[aligner::Member]) {
        if members.len() < 2 || !self.distinct_lines(members) {
            return;
        }
        aligner::emit_group(self.source, members, self.settings, &mut self.edits);
    }

    /// Builds an alignment member from a single annotated parameter,
    /// or returns `None` to signal a group break.
    fn parameter_member(&self, param_ref: AnyParameterRef<'_>) -> Option<aligner::Member> {
        let annotation = param_ref.annotation()?;
        aligner::line_anchored_member_at_kind(
            self.source,
            TextRange::new(param_ref.name().end(), annotation.start()),
            TokenKind::Colon,
        )
    }

    /// Processes consecutive `AnnAssign` statements in a class body,
    /// grouping them by line adjacency through the shared
    /// `aligner::line_adjacent_groups` primitive.
    fn process_class_fields(&mut self, body: &[Stmt]) {
        for members in
            aligner::line_adjacent_groups(self.source, body, |s| self.class_field_member(s))
        {
            self.emit(&members);
        }
    }

    fn process_dict(&mut self, d: &ExprDict) {
        if d.items.len() < 2 || self.comment_ranges.intersects(d.range()) {
            return;
        }
        let members: Vec<aligner::Member> = d
            .items
            .iter()
            .filter_map(|item| {
                let key = item.key.as_ref()?;
                aligner::line_anchored_member_at_kind(
                    self.source,
                    TextRange::new(key.end(), item.value.start()),
                    TokenKind::Colon,
                )
            })
            .collect();
        self.emit(&members);
    }

    /// Detects and aligns a Google/numpy-style `Args:` section in the
    /// body's leading docstring. Only the single-part string literal
    /// case is handled, leaving implicitly concatenated docstrings
    /// alone.
    fn process_docstring_args(&mut self, body: &[Stmt]) {
        let Some(lit) = body
            .first()
            .and_then(Stmt::as_expr_stmt)
            .and_then(|s| s.value.as_string_literal_expr())
            .filter(|lit| !lit.value.is_implicit_concatenated())
        else {
            return;
        };
        let members = self.docstring_args_members(lit.range());
        self.emit(&members);
    }

    fn process_parameters(&mut self, params: &Parameters) {
        let optional: Vec<Option<aligner::Member>> = params
            .iter_source_order()
            .map(|p| self.parameter_member(p))
            .collect();
        for run in optional.split(Option::is_none) {
            let group: Vec<aligner::Member> = run.iter().flatten().copied().collect();
            self.emit(&group);
        }
    }
}

impl<'a> AstVisitor<'a> for Visitor<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Dict(d) = expr {
            self.process_dict(d);
        }
        walk_expr(self, expr);
    }

    fn visit_parameters(&mut self, parameters: &'a Parameters) {
        self.process_parameters(parameters);
        walk_parameters(self, parameters);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(cd) => {
                self.process_class_fields(&cd.body);
                self.process_docstring_args(&cd.body);
            }
            Stmt::FunctionDef(fd) => {
                self.process_docstring_args(&fd.body);
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns the byte offset of `Args:` when it appears as a section
/// header, which means it sits at the start of a line (modulo leading
/// whitespace) and is followed only by whitespace on that line.
fn find_args_header(body: &str) -> Option<usize> {
    body.universal_newlines().find_map(|line| {
        let content = line.as_str();
        let stripped = content.trim_whitespace_start();
        let after = stripped.strip_prefix("Args:")?;
        after.trim_whitespace().is_empty().then(|| {
            let indent_len = content.len() - stripped.len();
            line.start().to_usize() + indent_len
        })
    })
}

/// Finds the byte offset of the `:` within a docstring entry line's
/// post-indent content. The pre-colon region may include the
/// argument name and an optional parenthesized type (e.g. `x (int)`).
/// Returns `None` when the line does not look like an entry.
fn find_entry_colon(stripped: &str) -> Option<usize> {
    let bytes = stripped.as_bytes();
    let &first = bytes.first()?;
    if !(first.is_ascii_alphabetic() || first == b'_' || first == b'*') {
        return None;
    }
    let mut paren_depth = 0usize;
    for (cursor, &b) in bytes.iter().enumerate() {
        match b {
            b'(' | b'[' => paren_depth += 1,
            b')' | b']' => paren_depth = paren_depth.saturating_sub(1),
            b':' if paren_depth == 0 => return Some(cursor),
            _ => {}
        }
    }
    None
}
