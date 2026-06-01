//! Capability advertisement and position-encoding negotiation.

use lsp_types::{
    ClientCapabilities, OneOf, PositionEncodingKind, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};
use ruff_source_file::PositionEncoding;

/// Picks the position encoding for a session: UTF-8 when the client
/// advertises it, otherwise UTF-16.
pub(super) fn negotiate_encoding(client: &ClientCapabilities) -> PositionEncoding {
    let advertised = client
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_ref());
    match advertised {
        Some(kinds) if kinds.contains(&PositionEncodingKind::UTF8) => PositionEncoding::Utf8,
        _ => PositionEncoding::Utf16,
    }
}

/// Builds the capabilities advertised in the `initialize` response:
/// full-document sync, a document-formatting provider, and the
/// negotiated position encoding.
pub(super) fn server_capabilities(encoding: PositionEncoding) -> ServerCapabilities {
    ServerCapabilities {
        document_formatting_provider: Some(OneOf::Left(true)),
        position_encoding: Some(encoding_kind(encoding)),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..ServerCapabilities::default()
    }
}

/// Maps the internal encoding onto the protocol's encoding token.
fn encoding_kind(encoding: PositionEncoding) -> PositionEncodingKind {
    match encoding {
        PositionEncoding::Utf8 => PositionEncodingKind::UTF8,
        PositionEncoding::Utf16 => PositionEncodingKind::UTF16,
        PositionEncoding::Utf32 => PositionEncodingKind::UTF32,
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::GeneralClientCapabilities;
    use rstest::rstest;

    use super::*;

    fn client_with(encodings: Option<Vec<PositionEncodingKind>>) -> ClientCapabilities {
        ClientCapabilities {
            general: Some(GeneralClientCapabilities {
                position_encodings: encodings,
                ..GeneralClientCapabilities::default()
            }),
            ..ClientCapabilities::default()
        }
    }

    #[test]
    fn capabilities_advertise_formatting_provider() {
        let caps = server_capabilities(PositionEncoding::Utf16);
        assert_eq!(caps.document_formatting_provider, Some(OneOf::Left(true)));
    }

    #[test]
    fn capabilities_advertise_full_document_sync() {
        let caps = server_capabilities(PositionEncoding::Utf16);
        assert_eq!(
            caps.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        );
    }

    #[rstest]
    #[case(PositionEncoding::Utf8, PositionEncodingKind::UTF8)]
    #[case(PositionEncoding::Utf16, PositionEncodingKind::UTF16)]
    #[case(PositionEncoding::Utf32, PositionEncodingKind::UTF32)]
    fn capabilities_echo_the_negotiated_encoding(
        #[case] encoding: PositionEncoding,
        #[case] kind: PositionEncodingKind,
    ) {
        let caps = server_capabilities(encoding);
        assert_eq!(caps.position_encoding, Some(kind));
    }

    #[test]
    fn negotiate_defaults_to_utf16_without_general_capabilities() {
        let encoding = negotiate_encoding(&ClientCapabilities::default());
        assert!(matches!(encoding, PositionEncoding::Utf16));
    }

    #[test]
    fn negotiate_defaults_to_utf16_without_utf8_advertised() {
        let encoding = negotiate_encoding(&client_with(Some(vec![PositionEncodingKind::UTF16])));
        assert!(matches!(encoding, PositionEncoding::Utf16));
    }

    #[test]
    fn negotiate_picks_utf8_when_advertised() {
        let encoding = negotiate_encoding(&client_with(Some(vec![
            PositionEncodingKind::UTF8,
            PositionEncodingKind::UTF16,
        ])));
        assert!(matches!(encoding, PositionEncoding::Utf8));
    }
}
