//! The `# prose: keep` marker a trailing comment carries, read off the
//! bracket rows of a dict or a dunder list and off a class header,
//! where it holds what it marks in the order written.

use ruff_python_ast::StmtClassDef;
use ruff_text_size::{Ranged, TextRange};

use super::end_rows_carry;
use crate::{source::Source, suppression::is_keep_marker};

/// True when the trailing comment on the `class` line of `class` or on
/// the line holding its header's closing `:` carries `# prose: keep`,
/// the marker that holds the statements of the class body in the order
/// written.
pub(crate) fn class_keeps_order(source: &Source, class: &StmtClassDef) -> bool {
    let colon_end = class
        .body
        .first()
        .map_or(class.end(), |first| source.prev_token_end(first.start()));
    has_keep_marker(source, TextRange::new(class.name.start(), colon_end))
}

/// True when the trailing comment on the line holding `span`'s start or
/// on the one holding its end carries `# prose: keep`, the marker read
/// across a dict's or a dunder list's brackets and across a class
/// header per [`class_keeps_order`].
pub(crate) fn has_keep_marker(source: &Source, span: impl Ranged) -> bool {
    end_rows_carry(source, span, is_keep_marker)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_class, parse};

    #[rstest]
    #[case::class_line("class C:  # prose: keep\n    b = 1\n    a = 2\n", true)]
    #[case::header_colon_line("class C(\n    Base,\n):  # prose: keep\n    b = 1\n", true)]
    #[case::wrapped_header_opener("class C(  # prose: keep\n    Base,\n):\n    b = 1\n", true)]
    #[case::inside_the_wrapped_header("class C(\n    Base,  # prose: keep\n):\n    b = 1\n", false)]
    #[case::decorator_line("@register  # prose: keep\nclass C:\n    b = 1\n", false)]
    #[case::first_body_line("class C:\n    b = 1  # prose: keep\n    a = 2\n", false)]
    fn class_keeps_order_reads_the_class_line_and_the_colon_line(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        assert_eq!(class_keeps_order(&s, first_class(&s)), expected);
    }
}
