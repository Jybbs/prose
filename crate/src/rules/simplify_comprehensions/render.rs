//! Turns a rewrite plan into the edits that land it.

use ruff_diagnostics::Edit;
use ruff_python_ast::{Expr, ExprTuple, Keyword, token::parenthesized_range};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{constructor::Constructor, plan::Plan};
use crate::{primitives::binding::sequence_elts, source::Source};

/// The edits `plan` lands over `expr`, or `None` where one of them would
/// cut through a comment or a dict entry carries its own grouping pair.
pub(super) fn edits_for(source: &Source, expr: &Expr, plan: &Plan) -> Option<Vec<Edit>> {
    let edits = match plan {
        Plan::Collapse { ctor, iter } => wrapping_edits(
            expr,
            iter.range(),
            padded(source, expr.start(), format!("{}(", ctor.as_str())),
            ")".to_owned(),
        ),
        Plan::Fixed(text) => vec![Edit::range_replacement(
            padded(source, expr.start(), (*text).to_owned()),
            expr.range(),
        )],
        Plan::Keywords(keywords) => keyword_edits(source, expr, keywords),
        Plan::Pairs { inner, pairs } => pair_edits(source, expr, inner, pairs)?,
        Plan::Rewrap { ctor, inner } => rewrap_edits(source, expr, *ctor, inner),
    };
    edits
        .iter()
        .all(|edit| !source.intersects_comment(edit.range()))
        .then_some(edits)
}

/// The byte width of the delimiter opening and closing `inner`, `0` for
/// a generator the enclosing call's own parentheses bound.
fn delimiter(inner: &Expr) -> TextSize {
    let paired = match inner {
        Expr::Dict(_)
        | Expr::DictComp(_)
        | Expr::List(_)
        | Expr::ListComp(_)
        | Expr::Set(_)
        | Expr::SetComp(_) => true,
        Expr::Generator(generator) => generator.parenthesized,
        Expr::Tuple(tuple) => tuple.parenthesized,
        _ => unreachable!("invariant: only a delimited argument reaches a rewrap"),
    };
    TextSize::from(u32::from(paired))
}

/// The edits rewriting `dict(a=1)` into its brace form, each keyword's
/// name becoming a quoted key.
fn keyword_edits(source: &Source, call: &Expr, keywords: &[Keyword]) -> Vec<Edit> {
    let (Some(first), Some(last)) = (keywords.first(), keywords.last()) else {
        unreachable!("invariant: the keyword plan carries at least one keyword")
    };
    let mut edits = vec![Edit::range_replacement(
        "{".to_owned(),
        TextRange::new(call.start(), first.start()),
    )];
    for keyword in keywords {
        let Some(name) = keyword.arg.as_ref() else {
            unreachable!("invariant: every keyword in the plan names an argument")
        };
        let value = source.paren_aware_range((&keyword.value).into(), keyword.into());
        edits.push(Edit::range_replacement(
            format!("\"{}\": ", name.as_str()),
            TextRange::new(keyword.start(), value.start()),
        ));
    }
    edits.push(Edit::range_replacement(
        "}".to_owned(),
        TextRange::new(last.end(), call.end()),
    ));
    edits
}

/// `text` carrying a leading space where the character before `start`
/// would otherwise run into it, as `return[x for x in xs]` does.
fn padded(source: &Source, start: TextSize, text: String) -> String {
    let joins = |c: char| c.is_alphanumeric() || c == '_';
    let merges = text.starts_with(joins)
        && source.text()[..start.to_usize()]
            .chars()
            .next_back()
            .is_some_and(joins);
    if merges { format!(" {text}") } else { text }
}

/// The edits rewriting a sequence of two-element tuples into a dict
/// literal or comprehension, each pair losing its parentheses and
/// trading its comma for a colon. Declines where a pair carries a
/// grouping pair beyond its own, whose opener the deletion would strand.
fn pair_edits(
    source: &Source,
    call: &Expr,
    inner: &Expr,
    pairs: &[&ExprTuple],
) -> Option<Vec<Edit>> {
    if pairs
        .iter()
        .any(|pair| parenthesized_range((*pair).into(), inner.into(), source.tokens()).is_some())
    {
        return None;
    }
    let width = delimiter(inner);
    let (open, close) = Constructor::Dict.brackets();
    let mut edits = vec![Edit::range_replacement(
        open.to_owned(),
        TextRange::new(call.start(), inner.start() + width),
    )];
    for pair in pairs {
        let [key, value] = pair.elts.as_slice() else {
            unreachable!("invariant: every pair in the plan holds two members")
        };
        if pair.parenthesized {
            edits.push(Edit::range_deletion(TextRange::new(
                pair.start(),
                key.start(),
            )));
            edits.push(Edit::range_deletion(TextRange::new(
                value.end(),
                pair.end(),
            )));
        }
        edits.push(Edit::range_replacement(
            ": ".to_owned(),
            TextRange::new(key.end(), value.start()),
        ));
    }
    edits.push(Edit::range_replacement(
        close.to_owned(),
        TextRange::new(inner.end() - width, call.end()),
    ));
    Some(edits)
}

/// The edits trading the call and `inner`'s own delimiters for `ctor`'s
/// pair. A one-element tuple's comma goes where the result is no longer
/// a tuple, and arrives where the result becomes one.
fn rewrap_edits(source: &Source, call: &Expr, ctor: Constructor, inner: &Expr) -> Vec<Edit> {
    let width = delimiter(inner);
    let (open, close) = ctor.brackets();
    let lone = sequence_elts(inner).is_some_and(|elts| elts.len() == 1);
    let comma = lone.then(|| source.trailing_comma(inner.range())).flatten();
    let kept_end = match comma {
        Some(comma) if ctor != Constructor::Tuple && inner.is_tuple_expr() => comma.start(),
        _ => inner.end() - width,
    };
    let close = if ctor == Constructor::Tuple && lone && comma.is_none() {
        format!(",{close}")
    } else {
        close.to_owned()
    };
    wrapping_edits(
        call,
        TextRange::new(inner.start() + width, kept_end),
        open.to_owned(),
        close,
    )
}

/// The head and tail edits leaving `kept` in place and replacing the
/// text on either side of it within `outer`.
fn wrapping_edits(outer: &Expr, kept: TextRange, open: String, close: String) -> Vec<Edit> {
    vec![
        Edit::range_replacement(open, TextRange::new(outer.start(), kept.start())),
        Edit::range_replacement(close, TextRange::new(kept.end(), outer.end())),
    ]
}
