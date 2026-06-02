//! Maps prose's byte-offset model onto the protocol's line/character
//! positions and diagnostic shape.

use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use ruff_source_file::PositionEncoding;
use ruff_text_size::TextSize;

use crate::{
    diagnostics::{Diagnostic, Severity},
    source::Source,
};

/// The range spanning the whole document, from the start to the position
/// past its final character.
pub(super) fn full_document_range(source: &Source, encoding: PositionEncoding) -> Range {
    Range {
        end: position_of(source, TextSize::of(source.text()), encoding),
        start: Position::default(),
    }
}

/// Renders a prose diagnostic as a protocol diagnostic, tagging it with
/// the rule slug and the `prose` source so the editor groups findings.
pub(super) fn to_lsp(
    source: &Source,
    diagnostic: &Diagnostic,
    encoding: PositionEncoding,
) -> LspDiagnostic {
    LspDiagnostic {
        code: Some(NumberOrString::String(diagnostic.rule.as_str().to_owned())),
        message: diagnostic.message.clone(),
        range: Range {
            end: position_of(source, diagnostic.range.end(), encoding),
            start: position_of(source, diagnostic.range.start(), encoding),
        },
        severity: Some(severity_of(diagnostic.severity)),
        source: Some("prose".to_owned()),
        ..LspDiagnostic::default()
    }
}

/// Maps a byte offset to a protocol position in the negotiated encoding.
fn position_of(source: &Source, offset: TextSize, encoding: PositionEncoding) -> Position {
    let location = source.source_location(offset, encoding);
    let clamp = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    Position {
        character: clamp(location.character_offset.to_zero_indexed()),
        line: clamp(location.line.to_zero_indexed()),
    }
}

/// Maps prose's severity onto the protocol's: format findings to
/// `INFORMATION`, lint findings to `WARNING`.
fn severity_of(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Format => DiagnosticSeverity::INFORMATION,
        Severity::Lint => DiagnosticSeverity::WARNING,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_support::parse;

    #[test]
    fn full_document_range_covers_a_multiline_buffer() {
        let range = full_document_range(&parse("a = 1\nbb = 2\n"), PositionEncoding::Utf16);
        assert_eq!(
            range.start,
            Position {
                character: 0,
                line: 0
            }
        );
        assert_eq!(
            range.end,
            Position {
                character: 0,
                line: 2
            }
        );
    }

    #[test]
    fn full_document_range_ends_past_the_last_char_without_newline() {
        let range = full_document_range(&parse("x = 1"), PositionEncoding::Utf16);
        assert_eq!(
            range.end,
            Position {
                character: 5,
                line: 0
            }
        );
    }

    #[test]
    fn full_document_range_of_empty_buffer_is_zero_width() {
        let range = full_document_range(&parse(""), PositionEncoding::Utf16);
        assert_eq!(range.start, range.end);
        assert_eq!(
            range.end,
            Position {
                character: 0,
                line: 0
            }
        );
    }

    #[test]
    fn position_of_counts_utf16_code_units_past_an_astral_char() {
        // The astral `😀` is one UTF-32 scalar, two UTF-16 code units,
        // and four UTF-8 bytes, so the column past it diverges by
        // encoding while the byte offset stays the same.
        let src = parse("x = \"😀y\"\n");
        let y_offset = TextSize::from(9); // byte offset of `y`
        assert_eq!(
            position_of(&src, y_offset, PositionEncoding::Utf16).character,
            7,
        );
        assert_eq!(
            position_of(&src, y_offset, PositionEncoding::Utf8).character,
            9,
        );
    }

    #[test]
    fn position_of_maps_line_starts() {
        let src = parse("a\nbb\nccc\n");
        assert_eq!(
            position_of(&src, TextSize::from(2), PositionEncoding::Utf16),
            Position {
                character: 0,
                line: 1
            },
        );
    }

    #[rstest]
    #[case(Severity::Format, DiagnosticSeverity::INFORMATION)]
    #[case(Severity::Lint, DiagnosticSeverity::WARNING)]
    fn severity_maps_each_variant(
        #[case] severity: Severity,
        #[case] expected: DiagnosticSeverity,
    ) {
        assert_eq!(severity_of(severity), expected);
    }
}
