//! Per-`Source` index of `# fmt: off` / `# fmt: on` / `# fmt: skip`
//! suppressed spans (plus the `# yapf: disable` / `# yapf: enable`
//! aliases). Built once during `Source` construction and consulted by
//! `Pipeline::run` to drop edits whose ranges overlap any span.

use ruff_python_trivia::{CommentLinePosition, CommentRanges, SuppressionKind};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

/// Sorted, non-overlapping list of byte ranges where format-affecting
/// rules must not emit edits. Range queries run in O(log n) over the
/// list.
#[derive(Debug)]
pub struct SuppressionMap {
    spans: Vec<TextRange>,
}

impl SuppressionMap {
    /// Walks `comments` against `source_text`, classifying each comment
    /// with `SuppressionKind` and folding the directive line offsets
    /// into the suppressed-span list. An unmatched `# fmt: off`
    /// extends through end of file. A stray `# fmt: on` is a no-op.
    /// Two consecutive `# fmt: off` directives flatten, with the first
    /// `# fmt: on` closing the block.
    pub fn from_comments(source_text: &str, comments: &CommentRanges) -> Self {
        let mut spans: Vec<TextRange> = Vec::new();
        let mut open_off: Option<TextSize> = None;
        for range in comments {
            let comment = &source_text[range];
            let Some(kind) = SuppressionKind::from_comment(comment) else {
                continue;
            };
            let position = CommentLinePosition::for_range(range, source_text);
            match kind {
                SuppressionKind::Off if position.is_own_line() => {
                    open_off.get_or_insert_with(|| source_text.line_start(range.start()));
                }
                SuppressionKind::On if position.is_own_line() => {
                    spans.extend(
                        open_off.take().map(|start| {
                            TextRange::new(start, source_text.line_start(range.start()))
                        }),
                    );
                }
                SuppressionKind::Skip => {
                    spans.push(source_text.full_line_range(range.start()));
                }
                _ => {}
            }
        }
        spans.extend(open_off.map(|start| TextRange::new(start, source_text.text_len())));
        Self {
            spans: merge(spans),
        }
    }

    /// Returns `true` when `ranged`'s span overlaps any suppressed
    /// span by at least one byte. Empty ranges report overlap when
    /// their offset strictly sits inside a span.
    pub fn intersects<R: Ranged>(&self, ranged: R) -> bool {
        let range = ranged.range();
        self.spans.binary_search_by(|s| s.ordering(range)).is_ok()
    }
}

fn merge(mut spans: Vec<TextRange>) -> Vec<TextRange> {
    spans.sort_unstable_by_key(|s| s.start());
    spans.dedup_by(|next, prev| {
        let overlaps = next.start() <= prev.end();
        if overlaps {
            *prev = prev.cover(*next);
        }
        overlaps
    });
    spans
}

#[cfg(test)]
mod tests {
    use crate::test_support::{parse, range};

    #[test]
    fn empty_source_yields_empty_map() {
        let source = parse("");
        let map = source.suppression_map();
        assert!(!map.intersects(range(0, 1)));
        assert!(!map.intersects(range(0, 0)));
    }

    #[test]
    fn intersects_catches_edit_that_fully_contains_a_span() {
        let text = "# fmt: off\nx = 1\n# fmt: on\n";
        let source = parse(text);
        let map = source.suppression_map();
        // Edit spanning the entire suppressed block (offsets 0..27)
        // overlaps the span and must be dropped.
        assert!(map.intersects(range(0, 27)));
    }
}
