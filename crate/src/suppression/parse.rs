//! The comment parser: the format and lint directives one comment's
//! text carries, read through the `# prose:` prefix scan and the
//! bracketed rule-list reader.

use itertools::{Either, Itertools};
use memchr::memchr;
use ruff_python_trivia::{PythonWhitespace, SuppressionKind};
use ruff_text_size::{TextLen, TextRange, TextSize};

use super::directive::{Named, RuleEntry};
use crate::rules::RuleId;

/// The format and lint directives one comment carries.
#[derive(Default)]
pub(super) struct Directives {
    pub(super) ignore: Option<Named>,
    pub(super) region: Option<SuppressionKind>,
    pub(super) skip: Option<Named>,
}

/// Each `#` chunk of `comment`, its range within the comment with its
/// trailing whitespace left out, beside whether it is a skip directive.
/// A `#` opening nothing but whitespace joins the chunk after it, so
/// `## prose: skip` reads as one chunk.
pub(super) fn chunks(comment: &str) -> Vec<(TextRange, bool)> {
    let mut chunks = Vec::new();
    let mut opened = None;
    let mut at = TextSize::default();
    let mut bodies = comment.split('#').skip(1).peekable();
    while let Some(body) = bodies.next() {
        let start = *opened.get_or_insert(at);
        at += '#'.text_len() + body.text_len();
        if body.trim_whitespace().is_empty() && bodies.peek().is_some() {
            continue;
        }
        let chunk = comment[TextRange::new(start, at)].trim_whitespace_end();
        chunks.push((TextRange::at(start, chunk.text_len()), is_skip(chunk)));
        opened = None;
    }
    chunks
}

/// Parses `comment` once for the three directive slots. Each `#` chunk
/// carrying the `prose:` prefix feeds the slot its body names, the
/// first `off` or `on` holding the region slot, a later `skip[<id>]`
/// or `ignore[<id>]` chunk unioning its ids into the first and a bare
/// one widening it. A `# fmt: skip` chunk widens the skip slot the way
/// a bare `# prose: skip` does, and the `# fmt:` and `# yapf:` region
/// aliases fill the region slot where no `prose:` directive took it. A
/// comment carrying no `:` holds nothing and skips the walk.
pub(super) fn directives(comment: &str) -> Directives {
    let mut found = Directives::default();
    if memchr(b':', comment.as_bytes()).is_none() {
        return found;
    }
    for body in prose_bodies(comment) {
        match body {
            "off" => {
                found.region.get_or_insert(SuppressionKind::Off);
            }
            "on" => {
                found.region.get_or_insert(SuppressionKind::On);
            }
            _ => {
                if let Some(named) = parse_entry(body, "skip") {
                    found.skip.get_or_insert_default().merge(named);
                }
            }
        }
        if let Some(named) = parse_entry(body, "ignore") {
            found.ignore.get_or_insert_default().merge(named);
        }
    }
    match SuppressionKind::from_comment(comment) {
        Some(kind @ (SuppressionKind::Off | SuppressionKind::On)) if found.region.is_none() => {
            found.region = Some(kind);
        }
        Some(SuppressionKind::Skip) => found.skip.get_or_insert_default().merge(Named::all()),
        _ => {}
    }
    found
}

/// True when `comment` is a recognized format or lint directive.
pub(crate) fn is_directive_comment(comment: &str) -> bool {
    let found = directives(comment);
    found.ignore.is_some() || found.region.is_some() || found.skip.is_some()
}

/// True when a `#` chunk of `comment` reads `prose: keep`, the marker
/// holding a dict, a dunder list, or a class body in the order written.
pub(crate) fn is_keep_marker(comment: &str) -> bool {
    prose_bodies(comment).any(|body| body == "keep")
}

/// True when the `#` chunk `chunk` is a `# prose: skip`, a
/// `# prose: skip[<id>]`, or a `# fmt: skip`.
fn is_skip(chunk: &str) -> bool {
    after_prose_prefix(chunk.trim_start_matches('#'))
        .is_some_and(|body| parse_entry(body, "skip").is_some())
        || matches!(
            SuppressionKind::from_comment(chunk),
            Some(SuppressionKind::Skip)
        )
}

/// Strips the leading `prose:` marker from `after_hash` and returns
/// the trimmed body. Returns `None` for any other shape.
fn after_prose_prefix(after_hash: &str) -> Option<&str> {
    after_hash
        .trim_whitespace()
        .strip_prefix("prose:")
        .map(str::trim_whitespace)
}

/// Parses the rule-id body of a `[<id>[, <id>...]]` suffix, splitting
/// the ids that name a registered rule from the ones that name none.
/// Returns `None` when the brackets are missing or malformed, and an
/// empty id between commas names nothing.
fn parse_bracketed_rule_list(body: &str) -> Option<Named> {
    let (known, unknown) = body
        .strip_prefix('[')?
        .strip_suffix(']')?
        .split(',')
        .map(str::trim_whitespace)
        .filter(|id| !id.is_empty())
        .partition_map(|id| match id.parse::<RuleId>() {
            Ok(rule) => Either::Left(rule),
            Err(_) => Either::Right(Box::from(id)),
        });
    Some(Named {
        entry: RuleEntry::Specific(known),
        unknown,
    })
}

/// The rules `body` names past `keyword`, every rule for the bare
/// keyword and the listed ones for a bracketed list. `None` for any
/// other shape.
fn parse_entry(body: &str, keyword: &str) -> Option<Named> {
    let rest = body.strip_prefix(keyword)?.trim_whitespace();
    if rest.is_empty() {
        return Some(Named::all());
    }
    parse_bracketed_rule_list(rest)
}

/// Yields the trimmed body past the `prose:` prefix of each `#` chunk
/// of `comment` that opens on it, in the order the chunks appear.
fn prose_bodies(comment: &str) -> impl Iterator<Item = &str> {
    comment.split('#').skip(1).filter_map(after_prose_prefix)
}
