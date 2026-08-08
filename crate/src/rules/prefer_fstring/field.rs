//! Renders one value into an f-string replacement field.

use std::borrow::Cow;

use ruff_python_ast::{Expr, ExprNumberLiteral, Number};

use crate::{primitives::unbracketed_colon, source::Source};

/// The characters a replacement field cannot carry below PEP 701. A
/// quote reuses the enclosing delimiter, a backslash is rejected
/// outright, and a brace closes the field early.
const BARRED: [char; 5] = ['"', '\'', '\\', '{', '}'];

/// Where a value sits inside the field it renders into.
#[derive(Clone, Copy)]
pub(super) enum Placement {
    /// An attribute or index follows the value, as in `{value.attr}`.
    Accessed,
    /// The value is the whole field, as in `{value}`.
    Whole,
}

/// The text `value` reads as inside a replacement field, `None` when
/// splicing it in would not parse. A comment, a line break, a starred
/// argument, any character from [`BARRED`], and an ungrouped `:` each
/// decline, the last of them covering a bare `lambda` whose colon the
/// field would read as its format spec.
pub(super) fn field_text<'src>(
    source: &'src Source,
    value: &Expr,
    placement: Placement,
) -> Option<Cow<'src, str>> {
    if matches!(value, Expr::Starred(_))
        || source.contains_line_break(value)
        || source.intersects_comment(value)
    {
        return None;
    }
    let text = source.slice(value);
    if text.contains(BARRED) || unbracketed_colon(text).is_some() {
        return None;
    }
    Some(if needs_parentheses(value, placement) {
        Cow::Owned(format!("({text})"))
    } else {
        Cow::Borrowed(text)
    })
}

/// True when `value` renders as a field only inside parentheses. A bare
/// generator and a walrus need them wherever they sit, whereas an
/// operator expression and a plain integer need them only under an
/// attribute or index. Every variant but the parenthesized generator
/// carries a range excluding its own grouping pair, so the wrap lands
/// once however many pairs the source wrote.
fn needs_parentheses(value: &Expr, placement: Placement) -> bool {
    match value {
        Expr::Generator(generator) => !generator.parenthesized,
        Expr::Named(_) => true,
        Expr::Await(_)
        | Expr::BinOp(_)
        | Expr::BoolOp(_)
        | Expr::Compare(_)
        | Expr::If(_)
        | Expr::UnaryOp(_)
        | Expr::NumberLiteral(ExprNumberLiteral {
            value: Number::Int(_),
            ..
        }) => matches!(placement, Placement::Accessed),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, parse};

    /// The field text `src` renders as, parsed as a whole value so a
    /// grouping pair the source wrote sits outside the expression range.
    fn rendered(src: &str, placement: Placement) -> Option<String> {
        let source = parse(&format!("X = {src}\n"));
        field_text(&source, first_value(&source), placement).map(Cow::into_owned)
    }

    #[rstest]
    #[case("value", "value")]
    #[case("obj.attr", "obj.attr")]
    #[case("table[0]", "table[0]")]
    #[case("a + b", "a + b")]
    #[case("a if cond else b", "a if cond else b")]
    #[case("call(1)", "call(1)")]
    #[case("-n", "-n")]
    #[case("12", "12")]
    #[case("(n := 5)", "(n := 5)")]
    #[case("(a for a in seq)", "(a for a in seq)")]
    fn field_text_leaves_a_whole_field_ungrouped(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(rendered(src, Placement::Whole).as_deref(), Some(expected));
    }

    #[rstest]
    #[case("a + b", "(a + b)")]
    #[case("12", "(12)")]
    #[case("-n", "(-n)")]
    #[case("await fetch()", "(await fetch())")]
    #[case("value", "value")]
    #[case("12.5", "12.5")]
    #[case("(a + b)", "(a + b)")]
    fn field_text_groups_an_accessed_operand(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(
            rendered(src, Placement::Accessed).as_deref(),
            Some(expected)
        );
    }

    #[rstest]
    fn field_text_declines_a_value_a_field_cannot_carry(
        #[values(
            "\"text\"",
            "'text'",
            "{1: 2}",
            "{1, 2}",
            "lambda: 1",
            "x if p else lambda: 1",
            "*rest"
        )]
        src: &str,
    ) {
        assert!(rendered(src, Placement::Whole).is_none(), "{src}");
    }

    #[rstest]
    #[case("((n := 5))", "(n := 5)")]
    #[case("((a for a in seq))", "(a for a in seq)")]
    fn field_text_wraps_once_however_many_pairs_the_source_wrote(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(rendered(src, Placement::Whole).as_deref(), Some(expected));
    }

    #[test]
    fn field_text_reads_a_slice_whose_colon_sits_inside_brackets() {
        assert_eq!(
            rendered("table[1:2]", Placement::Whole).as_deref(),
            Some("table[1:2]")
        );
    }

    #[test]
    fn field_text_declines_a_value_spanning_two_lines() {
        let source = parse("X = (\n    a\n    + b\n)\n");
        assert!(field_text(&source, first_value(&source), Placement::Whole).is_none());
    }
}
