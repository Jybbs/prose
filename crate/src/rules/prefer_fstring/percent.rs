//! Rewrites a printf-style `%` interpolation as an f-string.

use std::{borrow::Cow, slice, str::FromStr};

use ruff_python_ast::{DictItem, Expr, ExprBinOp, ExprDict, Operator};
use ruff_python_literal::cformat::{CFormatPart, CFormatSpec, CFormatString};
use ruff_python_stdlib::identifiers::is_identifier;

use crate::{
    primitives::effect::value_is_effectful,
    rules::prefer_fstring::{
        field::{Placement, field_text},
        literal::{Template, escape_braces},
        spec::suffix,
    },
    source::Source,
};

/// The f-string text `binop` rewrites to, `None` wherever the two forms
/// would not render alike.
pub(super) fn rewritten(source: &Source, binop: &ExprBinOp) -> Option<String> {
    if binop.op != Operator::Mod {
        return None;
    }
    let template = Template::read(source, binop.left.as_string_literal_expr()?)?;
    let parsed = CFormatString::from_str(template.body).ok()?;
    let specs: Vec<&CFormatSpec> = parsed
        .iter()
        .filter_map(|(_, part)| match part {
            CFormatPart::Spec(spec) => Some(spec),
            CFormatPart::Literal(_) => None,
        })
        .collect();
    if specs.is_empty() {
        return None;
    }
    let mut values = bind(source, &specs, &binop.right)?.into_iter();
    let mut body = String::with_capacity(template.body.len());
    for (_, part) in parsed.iter() {
        match part {
            CFormatPart::Literal(text) => body.push_str(&escape_braces(text, template.raw())),
            CFormatPart::Spec(spec) => {
                body.push('{');
                body.push_str(&values.next().expect("bind yields one value per spec"));
                body.push_str(&suffix(spec)?);
                body.push('}');
            }
        }
    }
    Some(template.wrap(&body))
}

/// The rendered value each spec reads, in spec order, `None` wherever
/// the right-hand side does not prove which value each spec reads.
///
/// A tuple literal binds by position and a dict literal of identifier
/// keys binds by name, and a lone spec also reads a literal right-hand
/// side. Every other shape declines.
fn bind<'src>(
    source: &'src Source,
    specs: &[&CFormatSpec],
    right: &Expr,
) -> Option<Vec<Cow<'src, str>>> {
    let all_keyed = specs.iter().all(|spec| spec.mapping_key.is_some());
    let none_keyed = specs.iter().all(|spec| spec.mapping_key.is_none());
    match right {
        Expr::Dict(dict) if all_keyed => bind_dict(source, specs, dict),
        Expr::Tuple(tuple) if none_keyed && tuple.elts.len() == specs.len() => {
            rendered_fields(source, &tuple.elts)
        }
        _ if none_keyed && specs.len() == 1 && right.as_literal_expr().is_some() => {
            rendered_fields(source, slice::from_ref(right))
        }
        _ => None,
    }
}

/// Binds each spec to the entry its mapping key names, `None` unless
/// the keys and the entries pair one to one under identifier-shaped
/// string keys. A key two specs read declines an effectful value.
fn bind_dict<'src>(
    source: &'src Source,
    specs: &[&CFormatSpec],
    dict: &ExprDict,
) -> Option<Vec<Cow<'src, str>>> {
    let mut entries: Vec<(&str, &Expr)> = Vec::with_capacity(dict.items.len());
    for DictItem { key, value } in &dict.items {
        let name = key.as_ref()?.as_string_literal_expr()?.value.to_str();
        let reads = specs
            .iter()
            .filter(|spec| spec.mapping_key.as_deref() == Some(name))
            .count();
        if !is_identifier(name)
            || reads == 0
            || entries.iter().any(|(seen, _)| *seen == name)
            || (reads > 1 && value_is_effectful(value))
        {
            return None;
        }
        entries.push((name, value));
    }
    let ordered = specs
        .iter()
        .map(|spec| {
            let key = spec.mapping_key.as_deref()?;
            entries
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| *value)
        })
        .collect::<Option<Vec<&Expr>>>()?;
    rendered_fields(source, ordered)
}

/// Every value rendered as a replacement field, in the order given,
/// `None` when any one of them cannot be carried.
fn rendered_fields<'src, 'ast>(
    source: &'src Source,
    values: impl IntoIterator<Item = &'ast Expr>,
) -> Option<Vec<Cow<'src, str>>> {
    values
        .into_iter()
        .map(|value| field_text(source, value, Placement::Whole))
        .collect()
}
