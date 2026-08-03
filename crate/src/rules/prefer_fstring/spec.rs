//! Translates one printf conversion spec into the text trailing a
//! replacement field.

use std::fmt::Write;

use ruff_python_literal::{
    cformat::{
        CConversionFlags, CFormatPrecision, CFormatQuantity, CFormatSpec, CFormatType, CNumberType,
    },
    format::FormatConversion,
};

/// The conversion and format spec trailing the value inside its field,
/// so `%r` yields `!r` and `%8.2f` yields `:8.2f`. `None` covers every
/// spec whose f-string counterpart renders differently.
///
/// A `%d`, `%i`, `%u`, or `%c` maps or truncates its value where the
/// matching presentation type raises, a `%b` belongs to bytes alone, a
/// `*` width reads its argument out of order, a precision on `%x`,
/// `%X`, or `%o` sets a digit floor rather than a cut, and a decorated
/// `%s`, `%r`, or `%a` measures the value rather than its rendered
/// text.
pub(super) fn suffix(spec: &CFormatSpec) -> Option<String> {
    if matches!(spec.min_field_width, Some(CFormatQuantity::FromValuesTuple))
        || matches!(
            spec.precision,
            Some(CFormatPrecision::Quantity(CFormatQuantity::FromValuesTuple))
        )
    {
        return None;
    }
    let bare = spec.flags.is_empty() && spec.min_field_width.is_none() && spec.precision.is_none();
    match spec.format_type {
        CFormatType::String(conversion) => match (bare, conversion) {
            (true, FormatConversion::Str) => Some(String::new()),
            (true, FormatConversion::Ascii) => Some("!a".to_owned()),
            (true, FormatConversion::Repr) => Some("!r".to_owned()),
            (_, FormatConversion::Bytes) | (false, _) => None,
        },
        CFormatType::Float(_) => Some(numeric_spec(spec)),
        CFormatType::Number(CNumberType::Hex(_) | CNumberType::Octal)
            if spec.precision.is_none() =>
        {
            Some(numeric_spec(spec))
        }
        CFormatType::Character | CFormatType::Number(_) => None,
    }
}

/// The format-spec flags `flags` translates to, dropping the blank sign
/// a sign character supersedes and the zero pad a left adjust does.
fn flag_text(flags: CConversionFlags) -> String {
    let mut text = String::new();
    if flags.intersects(CConversionFlags::LEFT_ADJUST) {
        text.push('<');
    }
    if flags.intersects(CConversionFlags::SIGN_CHAR) {
        text.push('+');
    } else if flags.intersects(CConversionFlags::BLANK_SIGN) {
        text.push(' ');
    }
    if flags.intersects(CConversionFlags::ALTERNATE_FORM) {
        text.push('#');
    }
    if flags.intersects(CConversionFlags::ZERO_PAD)
        && !flags.intersects(CConversionFlags::LEFT_ADJUST)
    {
        text.push('0');
    }
    text
}

/// The `:`-led format spec a numeric conversion carries.
fn numeric_spec(spec: &CFormatSpec) -> String {
    let mut text = format!(":{}", flag_text(spec.flags));
    if let Some(CFormatQuantity::Amount(width)) = spec.min_field_width {
        let _ = write!(text, "{width}");
    }
    match spec.precision {
        Some(CFormatPrecision::Quantity(CFormatQuantity::Amount(digits))) => {
            let _ = write!(text, ".{digits}");
        }
        Some(CFormatPrecision::Dot) => text.push_str(".0"),
        _ => {}
    }
    text.push(spec.format_char);
    text
}
