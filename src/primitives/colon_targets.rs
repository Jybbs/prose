//! Member constructors for the four `:` alignment contexts: dict
//! items, Pydantic-style class fields, annotated function parameters,
//! and Google/numpy docstring `Args:` entries. `align_colons` consumes
//! these to align multi-item groups, whereas `singleton_rule` consumes
//! the same shapes to strip pre-colon padding from singleton groups.

use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyParameterRef, DictItem, ExprDict, ExprStringLiteral, Parameters, Stmt};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::primitives::aligner;
use crate::source::Source;

/// Builds an alignment member for a class-body annotated assignment,
/// anchored on the `:` between target and annotation. Returns `None`
/// for any other statement shape.
pub fn class_field(source: &Source, stmt: &Stmt) -> Option<aligner::Member> {
    let ann = stmt.as_ann_assign_stmt()?;
    aligner::line_anchored_member_at_kind(
        source,
        TextRange::new(ann.target.end(), ann.annotation.start()),
        TokenKind::Colon,
    )
}

/// Walks `body`, qualifying each statement through `class_field`,
/// and returns one group per run of contiguous line-adjacent
/// annotated-assignment statements.
pub fn class_field_groups(source: &Source, body: &[Stmt]) -> Vec<Vec<aligner::Member>> {
    aligner::line_adjacent_groups(source, body, |s| class_field(source, s))
}

/// Builds an alignment member for a `key: value` dict entry, anchored
/// on the `:` between key and value. Returns `None` for `**spread`
/// entries that have no key.
pub fn dict_item(source: &Source, item: &DictItem) -> Option<aligner::Member> {
    let key = item.key.as_ref()?;
    aligner::line_anchored_member_at_kind(
        source,
        TextRange::new(key.end(), item.value.start()),
        TokenKind::Colon,
    )
}

/// Returns one alignment member per `key: value` entry in `d`.
/// `**spread` entries (no key) contribute nothing.
pub fn dict_members(source: &Source, d: &ExprDict) -> Vec<aligner::Member> {
    d.iter()
        .filter_map(|item| dict_item(source, item))
        .collect()
}

/// Returns one alignment member per entry in the body's leading
/// docstring's `Args:` section. Returns an empty `Vec` when the body
/// has no leading docstring, when the docstring is implicitly
/// concatenated, or when the docstring carries no `Args:` header.
/// An entry is any line whose first non-whitespace content runs up
/// to a `:` before the line ends. Continuation lines, blank lines,
/// and the next section header end the block.
pub fn docstring_args(source: &Source, body: &[Stmt]) -> Vec<aligner::Member> {
    let Some(string_literal) = body
        .first()
        .and_then(Stmt::as_expr_stmt)
        .and_then(|s| s.value.as_string_literal_expr())
        .and_then(ExprStringLiteral::as_single_part_string)
    else {
        return Vec::new();
    };
    let ds_range = string_literal.range();
    let text = source.slice(ds_range);
    let Some((header_offset, header_indent_len)) = find_args_header(text) else {
        return Vec::new();
    };

    let mut members = Vec::new();
    let mut entry_indent_len: Option<usize> = None;
    let after_header = header_offset + "Args:".len();
    for line in text[after_header..].universal_newlines().skip(1) {
        let content = line.as_str();
        let stripped = content.trim_whitespace_start();
        let line_indent_len = content.len() - stripped.len();

        if stripped.is_empty() || line_indent_len <= header_indent_len {
            break;
        }

        let expected = *entry_indent_len.get_or_insert(line_indent_len);
        if line_indent_len > expected {
            continue;
        }
        if line_indent_len < expected {
            break;
        }

        if let Some(colon_rel) = find_entry_colon(stripped) {
            let line_offset = TextSize::try_from(after_header + line_indent_len + colon_rel)
                .expect("docstring colon offset fits in TextSize");
            let colon_start = ds_range.start() + line.start() + line_offset;
            members.push(aligner::line_anchored_member(source, colon_start));
        }
    }
    members
}

/// Builds an alignment member for an annotated function parameter,
/// anchored on the `:` between name and annotation. Returns `None` for
/// unannotated parameters, signaling a group break to callers.
pub fn parameter(source: &Source, param: AnyParameterRef<'_>) -> Option<aligner::Member> {
    let annotation = param.annotation()?;
    aligner::line_anchored_member_at_kind(
        source,
        TextRange::new(param.name().end(), annotation.start()),
        TokenKind::Colon,
    )
}

/// Walks `params` in source order and returns one group per run of
/// contiguous annotated parameters, splitting at every unannotated
/// parameter (*the `self`/`cls` boundary, positional-only and
/// keyword-only separators when bare, anything else without an
/// annotation*).
pub fn parameter_groups(source: &Source, params: &Parameters) -> Vec<Vec<aligner::Member>> {
    let optional: Vec<Option<aligner::Member>> = params
        .iter_source_order()
        .map(|p| parameter(source, p))
        .collect();
    optional
        .split(Option::is_none)
        .map(|run| run.iter().flatten().copied().collect())
        .collect()
}

/// Returns `(byte_offset_of_args, line_indent_len)` for the first
/// `Args:` section header, which means a line whose first
/// non-whitespace content is exactly `Args:` followed only by
/// whitespace. The indent length is the byte count of leading
/// whitespace on the header's line, surfaced so callers can compare
/// subsequent entry-line indents without recomputing.
fn find_args_header(body: &str) -> Option<(usize, usize)> {
    body.universal_newlines().find_map(|line| {
        let content = line.as_str();
        let stripped = content.trim_whitespace_start();
        let after = stripped.strip_prefix("Args:")?;
        after.trim_whitespace().is_empty().then(|| {
            let indent_len = content.len() - stripped.len();
            (line.start().to_usize() + indent_len, indent_len)
        })
    })
}

/// Finds the byte offset of the `:` within a docstring entry line's
/// post-indent content. The pre-colon region may include the argument
/// name and an optional parenthesized type (e.g. `x (int)`). Returns
/// `None` when the line does not look like an entry.
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
