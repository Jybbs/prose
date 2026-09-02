//! PEP 723 inline-metadata reading. A standalone script the ancestor
//! walk never reaches still carries its `[tool.prose]` table in a
//! leading `# /// script` block.
//!
//! `ruff_python_ast::script::ScriptTag` parses the same block, but in
//! `0.15.10` its `metadata` field is private with no accessor, so the
//! extraction is reimplemented here against the PEP 723 grammar.

use memchr::memmem;
use ruff_source_file::UniversalNewlines;

use super::{ConfigError, load::prose_table_from_str};

/// Opening pragma of a PEP 723 metadata block.
const OPEN: &str = "# /// script";

/// Reads the `[tool.prose]` table from `bytes`'s leading PEP 723 block,
/// or `None` when the file carries no block, no closing pragma, or no
/// prose table.
///
/// # Errors
///
/// Returns `ConfigError::Toml` when the stripped block is not valid TOML.
pub(super) fn extract_prose_table(bytes: &[u8]) -> Result<Option<toml::Table>, ConfigError> {
    let Some(metadata) = open_line_offset(bytes).and_then(|open| {
        std::str::from_utf8(bytes)
            .ok()
            .and_then(|text| script_metadata(text, open))
    }) else {
        return Ok(None);
    };
    prose_table_from_str(&metadata)
}

/// The offset of the first line of `bytes` reading exactly `OPEN`,
/// closed by a line feed, a carriage return and line feed, or the end
/// of the bytes.
fn open_line_offset(bytes: &[u8]) -> Option<usize> {
    memmem::find_iter(bytes, OPEN).find(|&at| {
        (at == 0 || bytes[at - 1] == b'\n')
            && matches!(
                &bytes[at + OPEN.len()..],
                [] | [b'\n', ..] | [b'\r', b'\n', ..]
            )
    })
}

/// Strips the comment prefixes from the metadata between the opening
/// `# /// script` line at `open` and the last closing `# ///`, yielding
/// the embedded TOML. `None` when no closing pragma follows.
fn script_metadata(text: &str, open: usize) -> Option<String> {
    let mut body: Vec<&str> = text[open..]
        .universal_newlines()
        .skip(1)
        .map_while(|line| {
            let rest = line.as_str().strip_prefix('#')?;
            if rest.is_empty() {
                return Some("");
            }
            rest.strip_prefix(' ')
        })
        .collect();
    body.truncate(body.iter().rposition(|line| *line == "///")?);
    Some(body.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[test]
    fn block_after_shebang_is_read() {
        let source = indoc! {br"
            #!/usr/bin/env -S uv run --script
            # /// script
            # [tool.prose]
            # code-line-length = 100
            # ///

            x = 1
        "};

        let table = extract_prose_table(source)
            .expect("parses")
            .expect("a block");

        assert_eq!(table["code-line-length"].as_integer(), Some(100));
    }

    #[test]
    fn block_keeps_a_blank_comment_line() {
        let source = indoc! {br"
            # /// script
            # [tool.prose]
            #
            # code-line-length = 100
            # ///
        "};

        let table = extract_prose_table(source)
            .expect("parses")
            .expect("a block");

        assert_eq!(table["code-line-length"].as_integer(), Some(100));
    }

    #[test]
    fn block_without_closing_pragma_yields_none() {
        let source = indoc! {br"
            # /// script
            # [tool.prose]
            # code-line-length = 100

            x = 1
        "};

        assert_eq!(extract_prose_table(source).expect("parses"), None);
    }

    #[test]
    fn block_without_prose_table_yields_none() {
        let source = indoc! {br#"
            # /// script
            # requires-python = ">=3.11"
            # ///
        "#};

        assert_eq!(extract_prose_table(source).expect("parses"), None);
    }

    #[test]
    fn file_without_block_yields_none() {
        assert_eq!(extract_prose_table(b"x = 1\n").expect("parses"), None);
    }

    #[test]
    fn malformed_block_toml_is_an_error() {
        let source = indoc! {br"
            # /// script
            # [tool.prose
            # ///
        "};

        assert!(extract_prose_table(source).is_err());
    }

    #[test]
    fn non_utf8_bytes_yield_none() {
        assert_eq!(extract_prose_table(&[0xff, 0xfe]).expect("parses"), None);
    }

    #[rstest]
    #[case(b"x = 1  # /// script\n# ///\n", None)]
    #[case(b"# /// script\r\n# [tool.prose]\r\n# ///\r\n", Some(0))]
    #[case(b"# /// script\r# ///\n", None)]
    fn open_line_offset_reads_only_a_whole_line(
        #[case] bytes: &[u8],
        #[case] expected: Option<usize>,
    ) {
        assert_eq!(open_line_offset(bytes), expected);
    }

    #[test]
    fn unspaced_comment_ends_the_block() {
        let source = indoc! {br"
            # /// script
            # [tool.prose]
            #bad
            # ///
        "};

        assert_eq!(extract_prose_table(source).expect("parses"), None);
    }
}
