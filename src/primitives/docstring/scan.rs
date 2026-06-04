//! Line classification for a docstring body: fences, blanks, list
//! markers, and their continuations.

/// The classification of a docstring body line by the shared fence,
/// blank, and list scanner. Every variant but `Body` is terminal for
/// the line; `Body` hands the line to the walker's own dispatch.
pub(crate) enum LineScan {
    Blank,
    Body,
    Fence,
    InFence,
    ListContinuation,
    ListMarker,
}

/// The fence and list state a docstring walker carries across lines.
/// [`LineScanner::classify`] advances the state per line and returns
/// its [`LineScan`], leaving each walker to dispatch its own effect.
pub(crate) struct LineScanner {
    body_indent_chars: usize,
    in_fence: bool,
    list_indent: Option<usize>,
}

impl LineScanner {
    pub(crate) fn new(body_indent_chars: usize) -> Self {
        Self {
            body_indent_chars,
            in_fence: false,
            list_indent: None,
        }
    }

    pub(crate) fn body_indent_chars(&self) -> usize {
        self.body_indent_chars
    }

    pub(crate) fn classify(&mut self, trimmed: &str, indent_chars: usize) -> LineScan {
        if trimmed.starts_with("```") {
            self.in_fence = !self.in_fence;
            self.list_indent = None;
            return LineScan::Fence;
        }
        if self.in_fence {
            return LineScan::InFence;
        }
        if trimmed.is_empty() {
            self.list_indent = None;
            return LineScan::Blank;
        }
        if let Some(marker) = self.list_indent {
            if indent_chars > marker {
                return LineScan::ListContinuation;
            }
            self.list_indent = None;
        }
        if indent_chars >= self.body_indent_chars && is_list_marker(trimmed) {
            self.list_indent = Some(indent_chars);
            return LineScan::ListMarker;
        }
        LineScan::Body
    }
}

/// True when `trimmed` opens with a Markdown list marker (`-`, `*`,
/// or `+` followed by a space) or a numeric marker (one or more
/// digits followed by `. `). Used by the shared line scanner to
/// recognize verbatim-passthrough list items.
fn is_list_marker(trimmed: &str) -> bool {
    if trimmed
        .strip_prefix(['-', '*', '+'])
        .is_some_and(|rest| rest.starts_with(' '))
    {
        return true;
    }
    let after_digits = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
    after_digits.len() < trimmed.len() && after_digits.starts_with(". ")
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn is_list_marker_matches_dash_star_plus_and_numeric() {
        assert!(is_list_marker("- foo"));
        assert!(is_list_marker("* foo"));
        assert!(is_list_marker("+ foo"));
        assert!(is_list_marker("1. foo"));
        assert!(is_list_marker("12. foo"));
        assert!(!is_list_marker("foo"));
        assert!(!is_list_marker("-foo"));
        assert!(!is_list_marker(". foo"));
    }
}
