//! Packs members onto rows and reads the indent each row opens at.

use std::ops::Range;

use super::*;

/// Builds the one-per-line expansion `(\n<prefix>item<sep>\n…\n<indent>)`
/// for `count` items at `indent`. `render` writes item `i` into the
/// buffer, and `separator` writes `<sep>`. Items sit at [`item_indent`],
/// the closing `)` at `indent`.
pub(crate) fn explode_parens(
    newline: &str,
    indent: usize,
    count: usize,
    mut render: impl FnMut(&mut String, usize),
    separator: Separator,
) -> String {
    let prefix = " ".repeat(item_indent(indent));
    let mut out = String::from("(");
    for i in 0..count {
        out.push_str(newline);
        out.push_str(&prefix);
        render(&mut out, i);
        out.push_str(separator.after(i, count));
    }
    out.push_str(newline);
    out.push_str(&prefix[..indent]);
    out.push(')');
    out
}

/// The column an exploded construct opens its items at, one
/// `INDENT_STEP` past the `indent` its closing bracket lands on.
pub(crate) fn item_indent(indent: usize) -> usize {
    indent + INDENT_STEP
}

/// Greedily groups item indices into lines, each opening after
/// `prefix_width` and packing items joined by `separator_width` columns
/// up to `budget`. The first item on every line is always placed, so an
/// item whose own line overflows still lands rather than splitting away.
pub(crate) fn pack(
    widths: &[usize],
    prefix_width: usize,
    separator_width: usize,
    budget: usize,
) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut line_width = 0;
    for (i, &width) in widths.iter().enumerate() {
        if i == start {
            line_width = prefix_width + width;
        } else if line_width + separator_width + width <= budget {
            line_width += separator_width + width;
        } else {
            lines.push(start..i);
            start = i;
            line_width = prefix_width + width;
        }
    }
    lines.push(start..widths.len());
    lines
}

/// True for a `Dict`, `List`, `Set`, or parenthesized `Tuple` shape
/// the expand path canonicalizes, the [`is_multi_entry`] shapes and
/// the one-entry `Dict`. An empty or single-item collection otherwise
/// has nothing to flow.
pub(crate) fn requires_expand(expr: &Expr) -> bool {
    is_multi_entry(expr.into()) || expr.as_dict_expr().is_some_and(|dict| dict.len() == 1)
}

/// Splits `block` at its first line break when that opening line holds
/// its bracket alone, yielding the bracket and the body beneath. A
/// single-line block and one whose first line carries content beside
/// the bracket both return `None`.
pub(super) fn flush_bracket_open(block: &str) -> Option<&str> {
    let (open, body) = block.split_once(['\n', '\r'])?;
    (open.trim().len() == 1).then_some(body)
}
