//! The line grammar of a Google-style docstring section: what reads as
//! a Title-case heading and what reads as a `name: description` entry
//! head. Every predicate here takes a trimmed line and returns a
//! verdict, carrying no state across lines.

use std::sync::LazyLock;

use regex_lite::Regex;

use crate::primitives::unbracketed_colon;

static SECTION_HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Z][A-Za-z]*( [A-Z][A-Za-z]*)*:").expect("static pattern compiles")
});

/// A parsed `name: description` entry head. `colon` is the byte offset
/// of the separating `:` within the trimmed line, and `desc_start` is
/// the byte offset where the description begins.
pub(crate) struct EntryHead<'a> {
    pub(crate) colon: usize,
    pub(crate) desc_start: usize,
    pub(crate) name: &'a str,
}

/// True when `trimmed` opens with a Title-case word or multi-word
/// run with every word capitalized, immediately followed by `:`.
/// Trailing content after the `:` is permitted.
pub(crate) fn section_heading(trimmed: &str) -> bool {
    SECTION_HEADING.is_match(trimmed)
}

/// Parses `trimmed` as a sibling of the entry above it. `None` when the
/// line continues the entry above it instead, which covers every line
/// deeper than `section_body_indent` whatever its shape.
pub(crate) fn sibling_entry_head(
    indent_chars: usize,
    section_body_indent: usize,
    trimmed: &str,
) -> Option<EntryHead<'_>> {
    (indent_chars == section_body_indent)
        .then_some(trimmed)
        .and_then(entry_head)
}

/// True when `trimmed` is an entry head carrying a parenthesized type
/// group, the `name (type): description` shape.
pub(crate) fn typed_entry_head(trimmed: &str) -> bool {
    unbracketed_colon(trimmed).is_some_and(|colon| trimmed[..colon].trim_end().ends_with(')'))
        && entry_head(trimmed).is_some()
}

/// Parses `trimmed` as a Google-style `name: description` entry head,
/// allowing a leading `*` or `**` on the name and a balanced
/// parenthesized type group between the name and the `:` (e.g.
/// `markup (str): A string.`, `*args: payload.`). The returned name
/// excludes any `*`/`**` prefix. `None` for any line that does not
/// match the head shape or carries no description after the `:`.
fn entry_head(trimmed: &str) -> Option<EntryHead<'_>> {
    let colon = unbracketed_colon(trimmed)?;
    let head = trimmed[..colon].trim_end();
    let after_stars = head.trim_start_matches('*');
    if head.len() - after_stars.len() > 2 {
        return None;
    }
    let name_end = after_stars
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(after_stars.len());
    let name = &after_stars[..name_end];
    if !name.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let paren_type = after_stars[name_end..].trim();
    let has_type_group = paren_type.starts_with('(') && paren_type.ends_with(')');
    if !(paren_type.is_empty() || has_type_group) {
        return None;
    }
    let description = trimmed[colon + 1..]
        .strip_prefix(char::is_whitespace)?
        .trim_start();
    if description.is_empty() {
        return None;
    }
    Some(EntryHead {
        colon,
        desc_start: trimmed.len() - description.len(),
        name,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn name_and_start(head: Option<EntryHead<'_>>) -> Option<(&str, usize)> {
        head.map(|h| (h.name, h.desc_start))
    }

    #[test]
    fn entry_head_measures_past_parenthesized_type() {
        assert_eq!(
            name_and_start(entry_head("markup (str): a string.")),
            Some(("markup", 14)),
        );
        assert_eq!(
            name_and_start(entry_head("flag (bool): on or off")),
            Some(("flag", 13)),
        );
        assert_eq!(
            name_and_start(entry_head("records (List[Tuple[int, str]]): rows")),
            Some(("records", 33)),
        );
    }

    #[test]
    fn entry_head_rejects_lines_without_name_colon_shape() {
        assert!(entry_head("just prose with no colon").is_none());
        assert!(entry_head("name:no_space_after_colon").is_none());
        assert!(entry_head(": no name before colon").is_none());
        assert!(entry_head("name: ").is_none());
        assert!(entry_head("name (only: parens)").is_none());
        assert!(entry_head("two words (int): not an entry").is_none());
    }

    #[test]
    fn entry_head_reports_the_colon_offset_within_the_trimmed_line() {
        let head = entry_head("markup (str): a string.").expect("entry head parses");
        assert_eq!(head.colon, 12);
        assert_eq!(&"markup (str): a string."[head.colon..=head.colon], ":");
    }

    #[test]
    fn entry_head_returns_name_and_description_offset() {
        assert_eq!(name_and_start(entry_head("name: desc")), Some(("name", 6)));
        assert_eq!(name_and_start(entry_head("name : desc")), Some(("name", 7)));
        assert_eq!(
            name_and_start(entry_head("dotted.name: desc")),
            Some(("dotted.name", 13)),
        );
        assert_eq!(
            name_and_start(entry_head("123: digits-only name")),
            Some(("123", 5)),
        );
    }

    #[test]
    fn entry_head_strips_up_to_two_star_prefixes_from_the_name() {
        assert_eq!(
            name_and_start(entry_head("*args: payload")),
            Some(("args", 7)),
        );
        assert_eq!(
            name_and_start(entry_head("**kwargs: extra")),
            Some(("kwargs", 10)),
        );
        assert_eq!(
            name_and_start(entry_head("**kwargs  : extra")),
            Some(("kwargs", 12)),
        );
        assert!(entry_head("***nope: three stars").is_none());
    }

    #[test]
    fn section_heading_accepts_multi_word_title_case_with_colon() {
        assert!(section_heading("Other Parameters:"));
        assert!(section_heading("See Also:"));
        assert!(section_heading("Side Effects:"));
    }

    #[rstest]
    fn section_heading_accepts_title_case_word_with_colon(
        #[values(
            "Args:",
            "Attributes:",
            "Raises:",
            "Returns:",
            "Yields:",
            "Examples:",
            "Note:",
            "Warning:",
            "Arguments:",
            "Parameters:",
            "Inputs:",
            "Steps:",
            "Outputs:"
        )]
        heading: &str,
    ) {
        assert!(section_heading(heading));
    }

    #[test]
    fn section_heading_accepts_trailing_content_after_colon() {
        assert!(section_heading("Returns: int"));
        assert!(section_heading("Note: see below"));
    }

    #[test]
    fn section_heading_rejects_lowercase_start_or_missing_colon() {
        assert!(!section_heading("args:"));
        assert!(!section_heading("Args :"));
        assert!(!section_heading("Args"));
        assert!(!section_heading("Foo bar:"));
        assert!(!section_heading("1Args:"));
        assert!(!section_heading(": no name"));
    }

    #[test]
    fn sibling_entry_head_opens_only_at_the_section_body_indent() {
        assert_eq!(
            name_and_start(sibling_entry_head(8, 8, "name: desc")),
            Some(("name", 6)),
        );
        assert!(sibling_entry_head(9, 8, "name: desc").is_none());
        assert!(sibling_entry_head(4, 8, "name: desc").is_none());
        assert!(sibling_entry_head(8, 8, "just prose").is_none());
    }

    #[test]
    fn typed_entry_head_requires_a_parenthesized_type_group() {
        assert!(typed_entry_head("markup (str): a string."));
        assert!(typed_entry_head("records (List[Tuple[int, str]]): rows"));
        assert!(typed_entry_head("*args (int): payload"));
        assert!(!typed_entry_head("markup: a string."));
        assert!(!typed_entry_head("name (only: parens)"));
        assert!(!typed_entry_head("two words (int): not an entry"));
        assert!(!typed_entry_head("See https://example.com for details."));
        assert!(!typed_entry_head("just prose with no colon"));
    }
}
