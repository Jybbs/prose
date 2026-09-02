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
use ruff_python_ast::Stmt;
use ruff_python_parser::{ParseOptions, parse_cells_unchecked};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use self::{deltas::Deltas, slide::Slide, tokens::Reparsed, window::Window};
use crate::{primitives::slots::item_holding, rules::RuleId, source::Source};

/// The share of the woven text, in percent, past which the windows a
/// splice would reparse cost more than the whole-file parse they stand
/// in for, the merge over the token stream, the slide over every node
/// past the windows, and the walks over the windows growing with the
/// text where a fresh parse grows with it once.
const WINDOW_SHARE_PERCENT: u64 = 65;

/// The text length below which the share gate never applies, a parse
/// of that size costing less than the gate's own reading.
const GATED_FROM: TextSize = TextSize::new(4096);

/// The reparse of each window a rule's edits fell inside.
pub(crate) struct Splice(Vec<Reparsed>);

/// `range` moved to where the woven text `map` describes holds it, the
/// way the slide moves a statement a splice leaves standing, collapsed
/// to an empty range where the edits swallowed it whole.
#[cfg(test)]
pub(crate) fn slid_range(map: &SourceMap, range: TextRange) -> TextRange {
    let deltas = Deltas::new(map);
    let (start, end) = (
        deltas.shift_before(range.start()),
        deltas.shift(range.end()),
    );
    TextRange::new(start.min(end), end)
}

#[cfg(test)]
impl Splice {
    /// Each window's span in the buffer the source holds, ascending.
    pub(crate) fn held(&self) -> Vec<TextRange> {
        self.0.iter().map(|window| window.held).collect()
    }

    /// Each window's span in the woven text, ascending.
    pub(crate) fn slid(&self) -> Vec<TextRange> {
        self.0.iter().map(|window| window.slid).collect()
    }
}

impl Source {
    /// True where this source's tree and token stream equal those a
    /// whole-file parse of its own text produces, every range included.
    fn matches_a_fresh_parse(&self) -> bool {
        super::parse_typed_module(self.text(), self.source_type)
            .is_ok_and(|fresh| *self.ast() == *fresh.syntax() && self.tokens() == fresh.tokens())
    }

    /// The reparse of each statement `map` reports edited, or `None`
    /// where the splice does not apply and the caller takes its
    /// whole-file parse.
    ///
    /// A splice declines a window whose new text does not parse, a
    /// nested window landing as anything but the one statement filling
    /// it, an edit writing text no window reads, windows reaching past
    /// [`WINDOW_SHARE_PERCENT`] of a text [`GATED_FROM`] long or longer,
    /// and a notebook, whose cell boundaries a splice would have to
    /// recut. A module-body window lands as any count of statements,
    /// none included, and a window whose end moved to another depth
    /// hands the levels it moved to the `Dedent` run past it.
    pub(crate) fn splice_of(&self, text: &str, map: &SourceMap) -> Option<Splice> {
        if self.is_notebook() {
            return None;
        }
        let deltas = Deltas::new(map);
        let covered: Vec<Window> = window::covering(self, deltas.replaced())
            .into_iter()
            .map(|held| Window {
                held,
                slid: deltas.slide_window(held),
            })
            .collect();
        let covers = |written: TextRange| {
            item_holding(&covered, written.start())
                .is_some_and(|window| window.slid.contains_range(written))
        };
        if !deltas.written().all(covers) {
            return None;
        }
        let reparsed: u64 = covered
            .iter()
            .map(|window| u64::from(window.slid.len().to_u32()))
            .sum();
        if text.text_len() >= GATED_FROM
            && reparsed * 100 > u64::from(text.text_len().to_u32()) * WINDOW_SHARE_PERCENT
        {
            return None;
        }
        let options = ParseOptions::from(self.source_type);
        covered
            .into_iter()
            .map(|Window { held, slid }| {
                let parsed = parse_cells_unchecked(text, [slid], &options);
                let run = window::module_level(self, held);
                let filled = matches!(&parsed.syntax().body[..], [only] if only.range() == slid);
                if !parsed.has_valid_syntax() || !(run || filled) {
                    return None;
                }
                let fresh = parsed.tokens().before(slid.end()).to_vec();
                let delta =
                    window::net_indent(&fresh) - window::net_indent(self.tokens().in_range(held));
                let stmts: Vec<Stmt> = parsed.into_syntax().body.into_iter().collect();
                Some(Reparsed {
                    delta,
                    fresh,
                    held,
                    run,
                    slid,
                    stmts,
                })
            })
            .collect::<Option<Vec<_>>>()
            .map(Splice)
    }

    /// This source rewritten as `text`, with `splice`'s statements
    /// grafted in and everything outside a window slid past the edits.
    /// The tree moves into the result, so a caller wanting the binding
    /// table takes the slot before this call and one replaying a
    /// rejected batch rebuilds from the entry buffer.
    ///
    /// The padding walk moves with it, every entry outside the windows
    /// slid and the entries inside them walked afresh, the outcome
    /// reported under `rule`.
    ///
    /// [`splice_of`](Self::splice_of) declines every notebook, so this
    /// path carries no cell boundaries and no cell numbering.
    pub(crate) fn spliced(
        mut self,
        text: String,
        map: &SourceMap,
        splice: Splice,
        rule: RuleId,
    ) -> Self {
        let deltas = Deltas::new(map);
        let spliced = tokens::spliced(
            &self.tokens,
            self.text(),
            text.text_len(),
            &deltas,
            &splice.0,
        );
        let stranded = std::mem::take(&mut self.stranded_padding);
        let columns = std::mem::take(&mut self.columns);
        let held: Vec<TextRange> = splice.0.iter().map(|window| window.held).collect();
        let windows: Vec<TextRange> = splice.0.iter().map(|window| window.slid).collect();
        let mut ast = self.ast;
        let (runs, nested): (Vec<Reparsed>, Vec<Reparsed>) =
            splice.0.into_iter().partition(|window| window.run);
        Slide::new(
            &deltas,
            nested.into_iter().map(|mut window| {
                let stmt = window
                    .stmts
                    .pop()
                    .expect("a nested window holds the one statement filling it");
                (window.held, stmt)
            }),
            runs.into_iter().map(|window| (window.held, window.stmts)),
        )
        .over_module(&mut ast);
        let mut next = Self::from_parts(
            text,
            self.file.name(),
            self.source_type,
            CellOffsets::default(),
            ast,
            spliced,
        );
        next.rebuild_stranded_padding(stranded, map, &windows, rule);
        next.rebuild_columns(columns, map, &held, &windows, rule);
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
        primitives::padding::Stranding,
        rules::RuleId,
        testing::{parse, range, replacement, woven},
    };

    /// `text` rewritten by `edits`, spliced where the splice applies and
    /// `None` where it declines.
    fn splice(text: &str, edits: Vec<Edit>) -> Option<Source> {
        let (rewritten, map) = woven(text, edits);
        let source = parse(text);
        let splice = source.splice_of(&rewritten, &map)?;
        Some(source.spliced(rewritten, &map, splice, RuleId::from("align-equals")))
    }

    #[test]
    fn a_spliced_source_carries_the_binding_table_forward() {
        let (text, map) = woven("x = 1\ny = 2\n", vec![replacement("  ", 1, 2)]);
        let mut source = parse("x = 1\ny = 2\n");
        source.binding_analysis();
        let bindings = source.take_binding_analysis();
        let splice = source.splice_of(&text, &map).expect("the splice applies");

        let mut next = source.spliced(text, &map, splice, RuleId::from("align-equals"));
        next.inherit(bindings, &map, RuleId::from("align-equals"), true);

        assert!(next.assert_carried_bindings_are_fresh("the spliced source"));
    }

    #[rstest]
    #[case::a_gap_the_reparsed_statement_drops(
        "x = call( 1 )\ny = [ 2 ]\n",
        replacement("call(1)", 4, 13)
    )]
    #[case::a_gap_the_reparsed_statement_adds(
        "x = call(1)\ny = [ 2 ]\n",
        replacement("call( 1 )", 4, 11)
    )]
    #[case::a_gap_slid_past_the_window("x = 1\ny = [ 2 ]\n", replacement("11", 4, 5))]
    #[case::a_gap_ahead_of_the_window("x = [ 1 ]\ny = 2\n", replacement("22", 14, 15))]
    fn a_spliced_source_rebuilds_the_padding_walk_over_its_windows(
        #[case] text: &str,
        #[case] edit: Edit,
    ) {
        let stranding = Stranding::new(RuleId::from("strip-stranded-padding"), true);
        let (rewritten, map) = woven(text, vec![edit]);
        let source = parse(text);
        assert!(
            !source.stranded_padding(stranding).is_empty(),
            "the case holds padding to carry"
        );
        let splice = source
            .splice_of(&rewritten, &map)
            .expect("the splice applies");

        let next = source.spliced(rewritten, &map, splice, RuleId::from("reflow-calls"));

        assert!(next.assert_rebuilt_padding_is_fresh("the spliced source"));
    }

    #[test]
    fn spliced_declines_a_notebook() {
        let source = crate::testing::notebook(&["x = 1\n", "y = 2\n"]);
        let (text, map) = woven(source.text(), vec![replacement("11", 4, 5)]);

        assert!(source.splice_of(&text, &map).is_none());
    }

    #[rstest]
    #[case::a_nested_window_its_statement_does_not_fill(
        "def f():\n    x = 1\n",
        replacement("x = 1 ", 13, 18)
    )]
    #[case::a_nested_window_reparsing_to_two_statements(
        "def f():\n    x = 1\n",
        replacement("x = 1\n    y = 2", 13, 18)
    )]
    #[case::a_window_whose_new_text_does_not_parse("x = 1\ny = 2\n", replacement("x = (", 0, 5))]
    fn spliced_declines_an_edit_no_window_carries(#[case] text: &str, #[case] edit: Edit) {
        assert!(splice(text, vec![edit]).is_none());
    }

    #[rstest]
    #[case::a_blank_line_added_between_two_definitions(
        "def a():\n    pass\n\ndef b():\n    pass\n",
        Edit::insertion("\n".to_owned(), 18u32.into())
    )]
    #[case::two_siblings_reordered(
        "def a():\n    pass\n\ndef b():\n    pass\n",
        replacement("def b():\n    pass\n\ndef a():\n    pass\n", 0, 37)
    )]
    #[case::a_statement_deleted("x = 1\ny = 2\nz = 3\n", Edit::range_deletion(range(6, 12)))]
    #[case::a_window_its_edit_emptied("x = 1\ny = 2\n", Edit::range_deletion(range(0, 5)))]
    #[case::a_statement_trailing_a_space("x = 1\ny = 2\n", replacement("x = 1 ", 0, 5))]
    #[case::a_statement_inserted_ahead_of_the_first(
        "x = 1\ny = 2\n",
        Edit::insertion("a = 0\n".to_owned(), 0u32.into())
    )]
    #[case::a_statement_inserted_past_the_last(
        "x = 1\n",
        Edit::insertion("y = 2\n".to_owned(), 6u32.into())
    )]
    #[case::an_edit_spanning_two_statements("x = 1\ny = 2\n", replacement("a = 9\nb = 8", 0, 11))]
    #[case::a_window_reparsing_to_two_statements(
        "x = 1\nz = 3\n",
        replacement("x = 1\ny = 2", 0, 5)
    )]
    #[case::a_window_whose_last_line_widened_its_indent(
        "if a:\n    x = 1\ny = 2\n",
        replacement("        x = 1", 6, 15)
    )]
    #[case::a_module_run_whose_last_line_gained_a_level(
        "x = 1\ny = 2\nz = 3\n",
        replacement("if a:\n    z = 3\n    y = 2", 6, 17)
    )]
    #[case::a_module_run_whose_last_line_lost_two_levels(
        "if a:\n    if b:\n        x = 1\ny = 2\n",
        replacement("x = 1", 0, 29)
    )]
    #[case::a_nested_window_whose_next_sibling_sits_above_depth_zero(
        "def f():\n    if a:\n        x = 1\n    y = 2\n",
        replacement("x = 1", 13, 32)
    )]
    fn spliced_reparses_a_run_of_module_siblings(#[case] text: &str, #[case] edit: Edit) {
        let (rewritten, _) = woven(text, vec![edit.clone()]);

        let next = splice(text, vec![edit]).expect("the splice applies");

        assert_eq!(next.text(), rewritten);
        assert!(next.matches_a_fresh_parse());
    }

    /// A module of `GATED_FROM` bytes and more, one assignment per line.
    fn gated_module() -> String {
        "x = 1\n".repeat(GATED_FROM.to_usize() / 6 + 1)
    }

    #[test]
    fn spliced_declines_windows_past_the_share_of_a_gated_text() {
        let text = gated_module();
        let whole = TextRange::up_to(text.text_len());
        let edit = Edit::range_replacement(text.replace("x = 1", "y = 2"), whole);

        assert!(splice(&text, vec![edit]).is_none());
    }

    #[test]
    fn spliced_takes_a_window_inside_the_share_of_a_gated_text() {
        let text = gated_module();
        let edit = replacement("11", 4, 5);

        let next = splice(&text, vec![edit]).expect("the splice applies");

        assert!(next.text().starts_with("x = 11\n"));
        assert!(next.matches_a_fresh_parse());
    }

    #[test]
    fn spliced_takes_the_enclosing_statement_for_a_nested_gap_edit() {
        let text = "def f():\n    x = 1\n    y = 2\n";
        let (rewritten, map) = woven(text, vec![Edit::insertion("\n".to_owned(), 19u32.into())]);
        let source = parse(text);

        let splice = source
            .splice_of(&rewritten, &map)
            .expect("the splice applies");

        assert_eq!(splice.0.len(), 1);
        assert_eq!(splice.0[0].held, range(0, 29));
    }

    #[test]
    fn spliced_merges_every_window_a_batch_edits() {
        let edits = vec![replacement("11", 4, 5), replacement("33", 16, 17)];

        let next = splice("x = 1\ny = 2\nz = 3\n", edits).expect("the splice applies");

        assert_eq!(next.text(), "x = 11\ny = 2\nz = 33\n");
        assert!(next.matches_a_fresh_parse());
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
    #[case::an_insertion_at_the_statement_end("x = 1\ny = 2\n", range(5, 5), " + 2")]
    #[case::a_statement_following_a_block("if a:\n    x = 1\ny = 2\n", range(20, 21), "22")]
    #[case::a_compound_statement_as_the_window(
        "if a:\n    x = 1\nelse:\n    y = 2\n",
        range(3, 4),
        "bb"
    )]
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
    #[case::a_continuation_line_joined_onto_its_statement(
        "def f():\n    x = foo(\n        a)\n    y = 2\n",
        range(17, 32),
        "foo(a)"
    )]
    fn spliced_rewrites_the_edited_statement(
        #[case] text: &str,
        #[case] span: TextRange,
        #[case] content: &str,
    ) {
        let edit = Edit::range_replacement(content.to_owned(), span);

        let next = splice(text, vec![edit]).expect("the splice applies");

        assert!(next.text().contains(content));
        assert!(next.matches_a_fresh_parse());
    }
}
