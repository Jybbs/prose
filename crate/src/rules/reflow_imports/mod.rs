//! Lays out an import block one module per line with its members
//! gathered behind it. `split-multi-module` breaks a comma-joined
//! `import a, b` into one statement per module, `merge-members` gathers
//! every `from <module> import …` line of one import run onto one
//! statement carrying each member once, and a roster overrunning
//! `Config::import_line_length` splits into repeated-prefix lines
//! greedily packed to that budget, each row seated at the column
//! `align-imports` settles it to once the rules between the two have
//! laid the block out. A multi-line import stays untouched, and a lone
//! name whose own line overflows keeps it rather than splitting further.

use std::{borrow::Cow, ops::Range};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Stmt, StmtImport, StmtImportFrom, helpers::format_import_from, token::TokenKind,
};
use ruff_python_trivia::indentation_at_offset;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    config::Config,
    primitives::{
        aligner,
        edit::{apply_inline_edits, narrowed_replacement, whole_line_deletion},
        imports::IMPORT_KEYWORD_WIDTH,
        inline::display_width,
        layout::pack,
        scope::{scoped_body, sub_bodies},
    },
    rule::{Rule, RuleId},
    rules::band_constants::BandConstants,
    source::Source,
};

mod forecast;
mod runs;

pub(crate) use runs::Folds;
use runs::{MergeRuns, band_forecast, comments_beside, module_groups};

/// What joins two members sharing one line, written between them and
/// counted against the budget each line packs to.
const MEMBER_SEPARATOR: &str = ", ";

/// The rows a from-import packs into, each the members it carries
/// beside the gap it holds ahead of `import`.
pub(super) type Packing = Vec<(Range<usize>, usize)>;

pub(crate) struct ReflowImports {
    align_settings: Option<aligner::Settings>,
    bands: Option<BandConstants>,
    divides: bool,
    first_party: Vec<String>,
    group_imports: bool,
    import_line_length: usize,
    merge_members: bool,
    sorts: bool,
    split_multi_module: bool,
}

impl ReflowImports {
    pub(crate) const MESSAGE: &'static str = "lay out an import block one module per line";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let align = &config.rules.align_imports;
        let rules = &config.rules.reflow_imports;
        Self {
            // Forecast the aligned column only when `align-imports`
            // runs, under the settings that rule resolves within, so
            // the column the forecast names is one the capped run seats.
            align_settings: align.enabled.then(|| config.import_align_settings()),
            bands: band_forecast(config),
            divides: config.group_imports_enabled() && config.rules.space_statements.enabled,
            first_party: config.first_party(),
            group_imports: config.group_imports_enabled(),
            import_line_length: config.import_width(),
            merge_members: rules.merge_members,
            sorts: config.alphabetize_siblings_enabled(),
            split_multi_module: rules.split_multi_module,
        }
    }
}

impl Rule for ReflowImports {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut layout = Layout {
            groups: Vec::new(),
            newline: source.newline_str(),
            packings: FxHashMap::default(),
            rule: self,
            source,
        };
        layout.layout_scope(&source.ast().body, source.module_range(), true);
        layout.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Layout<'a> {
    groups: Vec<Vec<Edit>>,
    newline: &'static str,
    /// The forecast packing of every from-import the body under layout
    /// could split, keyed by statement start.
    packings: FxHashMap<TextSize, Packing>,
    rule: &'a ReflowImports,
    source: &'a Source,
}

impl<'a> Layout<'a> {
    /// The edit rewriting `node` to `rows` joined one per line at the
    /// statement's indent, `None` where the statement does not open its
    /// own line or already reads that way.
    fn joined_rows_edit(&self, node: &impl Ranged, rows: &[String]) -> Option<Edit> {
        let indent = own_line_indent(self.source, node)?;
        let joiner = format!("{}{indent}", self.newline);
        narrowed_replacement(self.source, node.range(), rows.join(&joiner))
    }

    /// Lays out `body` and then every body beneath it, a class or
    /// function suite leaving module scope so no band forecast reaches
    /// the imports inside it.
    fn layout_scope(&mut self, body: &'a [Stmt], outer: TextRange, module_scope: bool) {
        self.process_body(body, outer, module_scope);
        for stmt in body {
            let nested = module_scope && scoped_body(stmt).is_none();
            for (sub, sub_outer) in sub_bodies(stmt) {
                self.layout_scope(sub, sub_outer, nested);
            }
        }
    }

    /// Folds every member of `group` into its first statement, laying
    /// the gathered roster out under the shared head and clearing each
    /// folded member's line. A group whose members already read that way
    /// emits nothing.
    fn merge_group(&mut self, body: &'a [Stmt], group: &[usize]) {
        let [lead, .., last] = group else {
            unreachable!("invariant: a merge group holds two or more members");
        };
        let node = body[*lead]
            .as_import_from_stmt()
            .expect("a merge group holds `from`-imports alone");
        let names = self.roster(group_aliases(body, group));
        let mut edits: Vec<Edit> = self
            .rows(node, &names)
            .and_then(|rows| self.packed_edit(node, &names, &rows))
            .into_iter()
            .collect();
        edits.extend(
            group[1..]
                .iter()
                .map(|&slot| whole_line_deletion(self.source, body[slot].range())),
        );
        let span = self
            .source
            .full_lines_within_cell(TextRange::new(body[*lead].start(), body[*last].end()));
        if apply_inline_edits(self.source, span, &edits) != self.source.slice(span) {
            self.groups.push(edits);
        }
    }

    /// Emits the packed rewrite of `node` when its roster overruns the
    /// row it opens.
    fn pack_lone(&mut self, node: &'a StmtImportFrom) {
        let [_, _, ..] = node.names.as_slice() else {
            return;
        };
        let names = self.roster(node.names.iter());
        let Some(rows) = self.rows(node, &names).filter(|rows| rows.len() > 1) else {
            return;
        };
        self.groups
            .extend(self.packed_edit(node, &names, &rows).map(|edit| vec![edit]));
    }

    /// The edit rewriting `node` to carry `names` on `rows`, the head
    /// repeated on each row ahead of its gap and `import`. `None` when
    /// the statement does not open its own line or already reads that
    /// way.
    fn packed_edit(
        &self,
        node: &StmtImportFrom,
        names: &[&str],
        rows: &[(Range<usize>, Cow<'a, str>)],
    ) -> Option<Edit> {
        let head = import_head(node);
        let rows: Vec<String> = rows
            .iter()
            .map(|(range, gap)| {
                format!(
                    "{head}{gap}import {}",
                    names[range.clone()].join(MEMBER_SEPARATOR)
                )
            })
            .collect();
        self.joined_rows_edit(node, &rows)
    }

    /// Folds each repeated module in `body` into one statement and
    /// splits every comma-joined bare import, one fix group apiece. At
    /// module scope a repeated module gathers across the constants
    /// `band-constants` hoists from between its statements.
    fn process_body(&mut self, body: &'a [Stmt], outer: TextRange, module_scope: bool) {
        let rule = self.rule;
        let source = self.source;
        let runs = MergeRuns::of(
            rule.bands.as_ref().filter(|_| module_scope),
            source,
            body,
            outer,
            |runs| {
                rule.align_settings.is_some()
                    && (runs.len() > 1 || !rule.sorts || comments_beside(source, body, outer, runs))
            },
        );
        if runs.runs.is_empty() {
            return;
        }
        let groups = if rule.merge_members {
            module_groups(self.source, body, outer, &runs)
        } else {
            Vec::new()
        };
        self.packings = rule
            .align_settings
            .map_or_else(FxHashMap::default, |settings| {
                self.forecast(settings, body, outer, &runs, &groups)
            });
        let gathered: Vec<usize> = groups.iter().flatten().copied().collect();
        for group in &groups {
            self.merge_group(body, group);
        }
        for (slot, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Import(bare) if rule.split_multi_module => self.split_bare_import(bare),
                Stmt::ImportFrom(lone) if !gathered.contains(&slot) => self.pack_lone(lone),
                _ => {}
            }
        }
    }

    /// The de-duplicated source text of `aliases`, the member roster one
    /// module's rows share, ordered as `alphabetize-siblings` would leave
    /// it unless that rule is off.
    fn roster(&self, aliases: impl Iterator<Item = &'a Alias>) -> Vec<&'a str> {
        let mut names: Vec<&str> = aliases
            .map(|alias| self.source.slice(alias.range()))
            .unique()
            .collect();
        if self.rule.sorts {
            names.sort_unstable();
        }
        names
    }

    /// The rows `node` packs `names` into and the gap each row holds
    /// ahead of `import`, the forecast packing where `align-imports`
    /// runs and otherwise the roster packed from the keyword's own
    /// column with the gap the source wrote repeated on every row.
    /// `None` where the keyword opens a line of its own.
    fn rows(
        &self,
        node: &StmtImportFrom,
        names: &[&str],
    ) -> Option<Vec<(Range<usize>, Cow<'a, str>)>> {
        if let Some(packing) = self.packings.get(&node.start()) {
            return Some(
                packing
                    .iter()
                    .map(|(range, gap)| (range.clone(), Cow::Owned(" ".repeat(*gap))))
                    .collect(),
            );
        }
        let gap = import_keyword_gap(self.source, node)?;
        let widths: Vec<usize> = names.iter().map(|name| display_width(name)).collect();
        let prefix = self.source.line_indent_width(node.start())
            + display_width(&import_head(node))
            + display_width(gap)
            + IMPORT_KEYWORD_WIDTH;
        Some(
            pack(
                &widths,
                prefix,
                MEMBER_SEPARATOR.len(),
                self.rule.import_line_length,
            )
            .into_iter()
            .map(|range| (range, Cow::Borrowed(gap)))
            .collect(),
        )
    }

    /// Emits the one-statement-per-module rewrite of a comma-joined
    /// bare import.
    fn split_bare_import(&mut self, node: &StmtImport) {
        let [_, _, ..] = node.names.as_slice() else {
            return;
        };
        let rows: Vec<String> = node
            .names
            .iter()
            .map(|alias| format!("import {}", self.source.slice(alias.range())))
            .collect();
        self.groups
            .extend(self.joined_rows_edit(node, &rows).map(|edit| vec![edit]));
    }
}

/// Every alias the `from`-imports at `slots` of `body` carry, in slot
/// order.
fn group_aliases<'a>(body: &'a [Stmt], slots: &[usize]) -> impl Iterator<Item = &'a Alias> {
    slots
        .iter()
        .filter_map(|&slot| body[slot].as_import_from_stmt())
        .flat_map(|node| &node.names)
}

/// The `from <dots><module>` head each row of `node` repeats, with the
/// relative-import leading dots folded into it.
fn import_head(node: &StmtImportFrom) -> String {
    format!(
        "from {}",
        format_import_from(node.level, node.module.as_deref())
    )
}

/// The whitespace between `node`'s module and its `import` keyword, the
/// column `align-imports` pads the keyword to. `None` when the keyword
/// opens a line of its own.
fn import_keyword_gap<'src>(source: &'src Source, node: &StmtImportFrom) -> Option<&'src str> {
    let anchored = aligner::line_anchored_member_at_kind(
        source,
        node.start(),
        node.range(),
        TokenKind::Import,
    )?;
    Some(source.slice(anchored.gap))
}

/// The leading-whitespace prefix of `node`'s line when `node` is a
/// single-line statement beginning that line, or `None` when it spans
/// a line break or other code precedes it (a `;`-joined statement).
fn own_line_indent<'src>(source: &'src Source, node: &impl Ranged) -> Option<&'src str> {
    if source.contains_line_break(node.range()) {
        return None;
    }
    indentation_at_offset(node.start(), source.text())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// The rule with every facet on and a ten-column import budget,
    /// forecasting no aligned column.
    fn tight_rule() -> ReflowImports {
        ReflowImports {
            align_settings: None,
            bands: None,
            divides: false,
            first_party: Vec::new(),
            group_imports: true,
            import_line_length: 10,
            merge_members: true,
            sorts: true,
            split_multi_module: true,
        }
    }

    #[test]
    fn a_merge_leaves_a_statement_sharing_its_line_with_code() {
        let source = parse(
            "from pkg import alpha
from pkg import beta; x = 1
",
        );
        assert!(tight_rule().apply(&source).is_empty());
    }

    #[rstest]
    #[case("from a.b.c import x\n", "from a.b.c")]
    #[case("from . import x\n", "from .")]
    #[case("from .sub import x\n", "from .sub")]
    #[case("from ..pkg import x\n", "from ..pkg")]
    #[case("from typing     import x\n", "from typing")]
    fn import_head_folds_relative_dots_into_the_repeated_head(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let node = source.ast().body[0]
            .as_import_from_stmt()
            .expect("first statement is a from-import");
        assert_eq!(import_head(node), expected);
    }

    #[rstest]
    #[case("from pkg import x\n", Some(" "))]
    #[case("from pkg     import x\n", Some("     "))]
    #[case("from pkg\timport x\n", Some("\t"))]
    #[case("from . import x\n", Some(" "))]
    #[case("from pkg import (\n    x,\n)\n", Some(" "))]
    #[case("from pkg \\\n    import x\n", None)]
    fn import_keyword_gap_reads_the_spaces_before_the_keyword(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let source = parse(src);
        let node = source.ast().body[0]
            .as_import_from_stmt()
            .expect("first statement is a from-import");

        assert_eq!(import_keyword_gap(&source, node), expected);
    }

    #[test]
    fn multi_line_import_is_left_untouched() {
        let source = parse("from pkg import (\n    alpha,\n    beta,\n    gamma,\n)\n");
        assert!(tight_rule().apply(&source).is_empty());
    }

    #[test]
    fn semicolon_joined_bare_import_is_left_untouched() {
        let source = parse("x = 1; import os, sys\n");
        assert!(tight_rule().apply(&source).is_empty());
    }

    #[test]
    fn semicolon_joined_import_is_left_untouched() {
        let source = parse("x = 1; from pkg import alpha, beta, gamma\n");
        assert!(tight_rule().apply(&source).is_empty());
    }
}
