//! Shared text builders for one-per-line expansion.

use crate::primitives::INDENT_STEP;

/// Builds the one-per-line expansion `(\n<prefix>item,\n…\n<indent>)`
/// for `count` items at `indent`. `render` writes item `i` into the
/// buffer and `comma` decides whether item `i` carries a trailing
/// comma. Items sit one `INDENT_STEP` past `indent`, the closing `)`
/// at `indent`.
pub(crate) fn explode_parens(
    newline: &str,
    indent: usize,
    count: usize,
    mut render: impl FnMut(&mut String, usize),
    comma: impl Fn(usize) -> bool,
) -> String {
    let prefix = " ".repeat(indent + INDENT_STEP);
    let mut out = String::from("(");
    for i in 0..count {
        out.push_str(newline);
        out.push_str(&prefix);
        render(&mut out, i);
        if comma(i) {
            out.push(',');
        }
    }
    out.push_str(newline);
    out.extend(std::iter::repeat_n(' ', indent));
    out.push(')');
    out
}
