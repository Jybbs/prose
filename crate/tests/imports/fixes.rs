//! Which recorded fix reached a row or a binding, meaning the rows an edit
//! rewrote and the span one fix's edits reach as written and as they leave
//! it.

use std::ops::Range;

use itertools::Itertools;

use crate::records::EditRows;

/// Reports whether one fix's edits take a name out of the span they reach.
pub(crate) fn drops(edits: &[EditRows], name: &str, text: &str) -> bool {
    let (was, now) = rewritten(edits, text);
    holds_word(&was, name) && !holds_word(&now, name)
}

/// Reports whether some text holds a name as a whole word.
pub(crate) fn holds_word(haystack: &str, name: &str) -> bool {
    let inside = |c: char| c.is_alphanumeric() || c == '_';
    !name.is_empty()
        && haystack.match_indices(name).any(|(at, _)| {
            !haystack[..at].chars().next_back().is_some_and(inside)
                && !haystack[at + name.len()..]
                    .chars()
                    .next()
                    .is_some_and(inside)
        })
}

/// Reports whether one fix's edits touch a row or write a given line.
pub(crate) fn reaches(edits: &[EditRows], rows: &Range<usize>, line: &str) -> bool {
    edits.iter().any(|edit| {
        (edit.rows.start < rows.end && rows.start < edit.rows.end)
            || !line.is_empty() && edit.content.lines().any(|written| written.trim() == line)
    })
}

/// The lines one fix's edits reach, as written and as the edits leave them.
pub(crate) fn rewritten(edits: &[EditRows], text: &str) -> (String, String) {
    let spans: Vec<_> = edits
        .iter()
        .map(|edit| (edit.range.start, edit.range.end, edit.content.as_str()))
        .sorted()
        .collect();
    let (Some(first), Some(last)) = (
        spans.iter().map(|(start, ..)| *start).min(),
        spans.iter().map(|(_, end, _)| *end).max(),
    ) else {
        return (String::new(), String::new());
    };
    if last > text.len() {
        return (String::new(), String::new());
    }
    let low = text[..first].rfind('\n').map_or(0, |at| at + 1);
    let high = text[last..].find('\n').map_or(text.len(), |at| last + at);
    let mut edited = text.to_owned();
    for (start, end, content) in spans.into_iter().rev() {
        edited.replace_range(start..end, content);
    }
    let shifted = (high + edited.len()).saturating_sub(text.len());
    (
        text[low..high].to_owned(),
        edited[low..shifted.min(edited.len())].to_owned(),
    )
}
