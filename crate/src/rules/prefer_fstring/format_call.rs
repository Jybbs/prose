//! Rewrites a `str.format()` call as an f-string.

use std::{borrow::Cow, fmt::Write};

use ruff_python_ast::{Expr, ExprCall};
use ruff_python_literal::format::{
    FieldName, FieldNamePart, FieldType, FormatPart, FormatString, FromTemplate,
};

use crate::{
    primitives::effect::value_is_effectful,
    rules::prefer_fstring::{
        field::{Placement, field_text},
        literal::{Template, escape_braces},
    },
    source::Source,
};

/// One argument the call passes, alongside how many fields have read
/// it.
struct Argument<'src> {
    name: Option<&'src str>,
    reads: usize,
    value: &'src Expr,
}

/// The call's arguments in source order, positional before keyword,
/// holding the auto-numbering cursor and whether any field has named
/// an explicit index.
struct Arguments<'src> {
    auto: usize,
    manual: bool,
    slots: Vec<Argument<'src>>,
}

impl<'src> Arguments<'src> {
    /// Collects the call's arguments, `None` when it carries no
    /// argument or a `**` expansion, neither of which names the value
    /// a field reads.
    fn read(call: &'src ExprCall) -> Option<Self> {
        let slots = call
            .arguments
            .args
            .iter()
            .map(|value| {
                Some(Argument {
                    name: None,
                    reads: 0,
                    value,
                })
            })
            .chain(call.arguments.keywords.iter().map(|keyword| {
                Some(Argument {
                    name: Some(keyword.arg.as_ref()?.as_str()),
                    reads: 0,
                    value: &keyword.value,
                })
            }))
            .collect::<Option<Vec<_>>>()?;
        (!slots.is_empty()).then_some(Self {
            auto: 0,
            manual: false,
            slots,
        })
    }

    /// True when every argument is read by at least one field.
    fn every_slot_read(&self) -> bool {
        self.slots.iter().all(|slot| slot.reads > 0)
    }

    /// The argument `field` names, counting the read. `None` covers a
    /// field naming no argument and a template mixing automatic
    /// numbering with explicit indices, which `str.format` rejects.
    fn slot(&mut self, field: &FieldType) -> Option<&Argument<'src>> {
        let index = match field {
            FieldType::Auto => {
                self.auto += 1;
                self.auto - 1
            }
            FieldType::Index(index) => {
                self.manual = true;
                *index
            }
            FieldType::Keyword(name) => self
                .slots
                .iter()
                .position(|slot| slot.name == Some(name.as_str()))?,
        };
        if self.manual && self.auto > 0 {
            return None;
        }
        let slot = self.slots.get_mut(index)?;
        if slot.name.is_some() && matches!(field, FieldType::Auto | FieldType::Index(_)) {
            return None;
        }
        slot.reads += 1;
        Some(slot)
    }
}

/// The f-string text `call` rewrites to, `None` wherever the two forms
/// would not render alike.
pub(super) fn rewritten(source: &Source, call: &ExprCall) -> Option<String> {
    let attribute = call.func.as_attribute_expr()?;
    if attribute.attr.as_str() != "format" {
        return None;
    }
    let template = Template::read(source, attribute.value.as_string_literal_expr()?)?;
    let mut arguments = Arguments::read(call)?;
    let parsed = if template.raw() {
        FormatString::from_raw_str(template.body)
    } else {
        FormatString::from_str(template.body)
    }
    .ok()?;
    if !parsed
        .format_parts
        .iter()
        .any(|part| matches!(part, FormatPart::Field { .. }))
    {
        return None;
    }
    let mut body = String::with_capacity(template.body.len());
    for part in &parsed.format_parts {
        match part {
            FormatPart::Literal(text) => body.push_str(&escape_braces(text, template.raw())),
            FormatPart::Field {
                field_name,
                conversion_spec,
                format_spec,
            } => {
                body.push('{');
                body.push_str(&rendered(source, &mut arguments, field_name)?);
                if let Some(conversion) = conversion_spec {
                    if !matches!(conversion, 'a' | 'r' | 's') {
                        return None;
                    }
                    let _ = write!(body, "!{conversion}");
                }
                if !format_spec.is_empty() {
                    if format_spec.contains('{') {
                        return None;
                    }
                    let _ = write!(body, ":{format_spec}");
                }
                body.push('}');
            }
        }
    }
    arguments.every_slot_read().then(|| template.wrap(&body))
}

/// The value text and trailing accessors the field named `field_name`
/// renders, `None` wherever the rewrite would not render alike. A
/// string index and a repeated effectful argument both decline.
fn rendered<'src>(
    source: &'src Source,
    arguments: &mut Arguments,
    field_name: &str,
) -> Option<Cow<'src, str>> {
    let field = FieldName::parse(field_name).ok()?;
    let slot = arguments.slot(&field.field_type)?;
    if slot.reads > 1 && value_is_effectful(slot.value) {
        return None;
    }
    if field.parts.is_empty() {
        return field_text(source, slot.value, Placement::Whole);
    }
    let mut text = field_text(source, slot.value, Placement::Accessed)?.into_owned();
    for part in &field.parts {
        match part {
            FieldNamePart::Attribute(name) => {
                let _ = write!(text, ".{name}");
            }
            FieldNamePart::Index(index) => {
                let _ = write!(text, "[{index}]");
            }
            FieldNamePart::StringIndex(_) => return None,
        }
    }
    Some(Cow::Owned(text))
}
