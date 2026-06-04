//! The docstring `Args:` colon context: aligns each `name:`
//! entry in a Google-style argument section.

use ruff_python_ast::{ExprStringLiteral, Stmt};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{Ranged, TextSize};

use crate::{primitives::aligner, source::Source};

/// Returns one alignment member per entry in the body's leading
/// docstring's `Args:` section. Returns an empty `Vec` when the body
/// has no leading docstring, when the docstring is implicitly
/// concatenated, or when the docstring carries no `Args:` header.
/// An entry is any line whose first non-whitespace content runs up
/// to a `:` before the line ends. Continuation lines, blank lines,
/// and the next section header end the block.
pub(super) fn docstring_args(source: &Source, body: &[Stmt]) -> Vec<aligner::Member> {
    let Some(string_literal) = body
        .first()
        .and_then(Stmt::as_expr_stmt)
        .and_then(|s| s.value.as_string_literal_expr())
        .and_then(ExprStringLiteral::as_single_part_string)
    else {
        return Vec::new();
    };
    let text = source.slice(string_literal);
    let mut lines = text.universal_newlines();
    let Some(header_indent_len) = lines.find_map(|line| {
        let stripped = line.trim_whitespace_start();
        let after = stripped.strip_prefix("Args:")?;
        after
            .trim_whitespace()
            .is_empty()
            .then_some(line.len() - stripped.len())
    }) else {
        return Vec::new();
    };

    let mut members = Vec::new();
    let mut entry_indent_len: Option<usize> = None;
    for line in lines {
        let stripped = line.trim_whitespace_start();
        let line_indent_len = line.len() - stripped.len();

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
            let colon_start = string_literal.start()
                + line.start()
                + TextSize::of(&line[..line_indent_len + colon_rel]);
            members.push(aligner::line_anchored_member(source, colon_start));
        }
    }
    members
}

/// Finds the byte offset of the `:` within a docstring entry line's
/// post-indent content. The pre-colon region may include the argument
/// name and an optional parenthesized type (e.g. `x (int)`). Returns
/// `None` when the line does not look like an entry.
fn find_entry_colon(stripped: &str) -> Option<usize> {
    let first = stripped.bytes().next()?;
    if !(first.is_ascii_alphabetic() || first == b'_' || first == b'*') {
        return None;
    }
    let mut paren_depth = 0usize;
    for (cursor, b) in stripped.bytes().enumerate() {
        match b {
            b'(' | b'[' => paren_depth += 1,
            b')' | b']' => paren_depth = paren_depth.saturating_sub(1),
            b':' if paren_depth == 0 => return Some(cursor),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_entry_colon_accepts_star_and_double_star() {
        assert_eq!(find_entry_colon("*args: list"), Some(5));
        assert_eq!(find_entry_colon("**kwargs: dict"), Some(8));
    }

    #[test]
    fn find_entry_colon_accepts_underscore_led_name() {
        assert_eq!(find_entry_colon("_arg: int"), Some(4));
        assert_eq!(find_entry_colon("_: int"), Some(1));
    }

    #[test]
    fn find_entry_colon_rejects_non_identifier_first_char() {
        assert!(find_entry_colon("1arg: int").is_none());
        assert!(find_entry_colon(": orphan").is_none());
        assert!(find_entry_colon("").is_none());
    }

    #[test]
    fn find_entry_colon_returns_none_when_no_top_level_colon() {
        assert!(find_entry_colon("argname only").is_none());
        assert!(find_entry_colon("name (only: parens)").is_none());
    }

    #[test]
    fn find_entry_colon_skips_colons_inside_parens_and_brackets() {
        assert_eq!(
            find_entry_colon("x (Dict[str, int]): mapping"),
            Some("x (Dict[str, int])".len()),
        );
    }
}
