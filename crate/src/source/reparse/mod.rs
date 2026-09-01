//! Reparses only the statements a rule's edits reached and splices them
//! into the tree and token stream the source already holds.
//!
//! `parse_cells_unchecked` parses a range of a buffer and keeps that
//! range's offsets, so a window's statements and tokens arrive already
//! placed. Everything outside a window slides by its edits' delta.

mod deltas;
mod flags;
mod slide;
mod tokens;
mod window;

use ruff_diagnostics::SourceMap;
use ruff_notebook::CellOffsets;
use ruff_python_parser::{ParseOptions, parse_cells_unchecked};
use ruff_text_size::{Ranged, TextRange};

use self::{deltas::Deltas, slide::Slide, tokens::Reparsed, window::Window};
use crate::{primitives::slots::item_holding, source::Source};

/// The reparse of each window a rule's edits fell inside.
pub(crate) struct Splice(Vec<Reparsed>);

impl Source {
    /// True where this source's tree and token stream equal those a
    /// whole-file parse of its own text produces, every range included.
    fn matches_a_fresh_parse(&self) -> bool {
        let Ok(fresh) = super::parse_typed_module(self.text(), self.source_type) else {
            return false;
        };
        *self.ast() == *fresh.syntax() && self.tokens() == fresh.tokens()
    }

    /// The reparse of each statement `map` reports edited, or `None`
    /// where the splice does not apply and the caller takes its
    /// whole-file parse.
    ///
    /// A splice declines an edit no single statement covers, a window
    /// whose new text does not parse, a window landing as anything but
    /// the one statement filling it, a window whose closing indent
    /// moved, an edit writing text no window reads, and a notebook,
    /// whose cell boundaries a splice would have to recut.
    pub(crate) fn splice_of(&self, text: &str, map: &SourceMap) -> Option<Splice> {
        if self.is_notebook() {
            return None;
        }
        let deltas = Deltas::new(map);
        let covered: Vec<Window> = window::covering(self, deltas.replaced())?
            .into_iter()
            .map(|held| Window {
                held,
                slid: deltas.slide(held),
            })
            .collect();
        let covers = |written: TextRange| {
            item_holding(&covered, written.start())
                .is_some_and(|window| window.slid.contains_range(written))
        };
        if !deltas.written().all(covers) {
            return None;
        }
        let options = ParseOptions::from(self.source_type);
        let mut windows = Vec::with_capacity(covered.len());
        for Window { held, slid } in covered {
            let parsed = parse_cells_unchecked(text, [slid], &options);
            if !parsed.has_valid_syntax() {
                return None;
            }
            let fresh = parsed.tokens().clone();
            let mut body = parsed.into_syntax().body;
            let stmt = body.pop()?;
            if !body.is_empty()
                || stmt.range() != slid
                || window::closing_indent(self.text(), held) != window::closing_indent(text, slid)
            {
                return None;
            }
            windows.push(Reparsed { fresh, held, stmt });
        }
        Some(Splice(windows))
    }

    /// This source rewritten as `text`, with `splice`'s statements
    /// grafted in and everything outside a window slid past the edits.
    ///
    /// [`splice_of`](Self::splice_of) declines every notebook, so this
    /// path carries no cell boundaries and no cell numbering.
    pub(crate) fn spliced(self, text: String, map: &SourceMap, splice: Splice) -> Self {
        let deltas = Deltas::new(map);
        let name = self.file.name().to_owned();
        let spliced = tokens::spliced(&self.tokens, self.text(), &deltas, &splice.0);
        let mut ast = self.ast;
        let grafts = splice
            .0
            .into_iter()
            .map(|window| (window.stmt.range(), window.stmt));
        Slide::new(&deltas, grafts).over_module(&mut ast);
        let next = Self::from_parts(
            text,
            name,
            self.source_type,
            CellOffsets::default(),
            ast,
            spliced,
        );
        debug_assert!(
            next.matches_a_fresh_parse(),
            "the spliced tree and token stream differ from a parse of the same text",
        );
        next
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_diagnostics::Edit;

    use super::*;
    use crate::{
        primitives::edit::apply_edits_mapped,
        testing::{parse, range},
    };

    /// `text` rewritten by `edits`, spliced where the splice applies and
    /// `None` where it declines.
    fn splice(text: &str, edits: Vec<Edit>) -> Option<Source> {
        let (woven, map) = apply_edits_mapped(text, edits).expect("the edits weave");
        let source = parse(text);
        let splice = source.splice_of(&woven, &map)?;
        Some(source.spliced(woven, &map, splice))
    }

    #[test]
    fn spliced_declines_a_notebook() {
        let source = crate::testing::notebook(&["x = 1\n", "y = 2\n"]);
        let edit = Edit::range_replacement("11".to_owned(), range(4, 5));
        let (woven, map) = apply_edits_mapped(source.text(), vec![edit]).expect("the edits weave");

        assert!(source.splice_of(&woven, &map).is_none());
    }

    #[test]
    fn spliced_declines_a_window_its_edit_emptied() {
        let edit = Edit::range_deletion(range(0, 5));

        assert!(splice("x = 1\ny = 2\n", vec![edit]).is_none());
    }

    #[test]
    fn spliced_declines_a_window_its_statement_does_not_fill() {
        let edit = Edit::range_replacement("x = 1 ".to_owned(), range(0, 5));

        assert!(splice("x = 1\ny = 2\n", vec![edit]).is_none());
    }

    #[test]
    fn spliced_declines_a_window_reparsing_to_two_statements() {
        let edit = Edit::range_replacement("x = 1\ny = 2".to_owned(), range(0, 5));

        assert!(splice("x = 1\nz = 3\n", vec![edit]).is_none());
    }

    #[test]
    fn spliced_declines_a_window_whose_closing_indent_moved() {
        let edit = Edit::range_replacement("        x = 1".to_owned(), range(6, 15));

        assert!(splice("if a:\n    x = 1\ny = 2\n", vec![edit]).is_none());
    }

    #[test]
    fn spliced_declines_a_window_whose_new_text_does_not_parse() {
        let edit = Edit::range_replacement("x = (".to_owned(), range(0, 5));

        assert!(splice("x = 1\ny = 2\n", vec![edit]).is_none());
    }

    #[test]
    fn spliced_declines_an_edit_no_single_statement_covers() {
        let edit = Edit::range_replacement("a = 9\nb = 8".to_owned(), range(0, 11));

        assert!(splice("x = 1\ny = 2\n", vec![edit]).is_none());
    }

    #[test]
    fn spliced_declines_an_insertion_writing_a_statement_of_its_own() {
        let edit = Edit::insertion("a = 0\n".to_owned(), 0u32.into());

        assert!(splice("x = 1\ny = 2\n", vec![edit]).is_none());
    }

    #[rstest]
    #[case::a_module_level_statement("x = 1\ny = 2\n", range(4, 5), "11")]
    #[case::a_statement_nested_in_a_definition("def f():\n    y = 2\nz = 3\n", range(17, 18), "22")]
    #[case::a_statement_holding_a_string("s = 'a'\nt = 'b'\n", range(4, 7), "'aaa'")]
    #[case::an_else_clause_beside_the_edit(
        "if a:\n    x = 1\nelse:\n    y = 2\n",
        range(30, 31),
        "22"
    )]
    #[case::an_f_string_format_spec_beside_the_edit(
        "a = 1\nb = f\"{a:>{a}}\"\n",
        range(4, 5),
        "111"
    )]
    #[case::a_module_of_one_statement_with_no_trailing_newline("x = 1", range(4, 5), "11")]
    #[case::global_and_nonlocal_slid_past_the_edit(
        "x = 1\ndef outer():\n    global x\n    y = 0\n    def inner():\n        nonlocal y\n        y = 2\n",
        range(4, 5),
        "11"
    )]
    #[case::match_patterns_slid_past_the_edit(
        "x = 1\nmatch x:\n    case [1, *rest]:\n        pass\n    case {\"k\": v, **extra}:\n        pass\n    case other:\n        pass\n",
        range(4, 5),
        "11"
    )]
    #[case::type_parameters_slid_past_the_edit(
        "x = 1\ndef f[T, *Ts, **P](a: T) -> T:\n    return a\n",
        range(4, 5),
        "11"
    )]
    fn spliced_rewrites_the_edited_statement(
        #[case] text: &str,
        #[case] span: TextRange,
        #[case] replacement: &str,
    ) {
        let edit = Edit::range_replacement(replacement.to_owned(), span);

        let next = splice(text, vec![edit]).expect("the splice applies");

        assert!(next.text().contains(replacement));
        assert!(next.matches_a_fresh_parse());
    }
}
