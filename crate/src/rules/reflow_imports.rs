//! Lays out an import block one module per line with its members
//! gathered behind it. `split-multi-module` breaks a comma-joined
//! `import a, b` into one statement per module, `merge-members` gathers
//! every `from <module> import …` line of one import run onto one
//! statement carrying each member once, and a roster overrunning
//! `Config::import_line_length` splits into repeated-prefix lines
//! greedily packed to that budget, or to the aligned prefix when
//! `align-imports` pads the `import` keyword rightward. A multi-line
//! import stays untouched, and a lone name whose own line overflows
//! keeps it rather than splitting further.

use std::collections::HashMap;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Stmt, StmtImport, StmtImportFrom,
    helpers::format_import_from,
    statement_visitor::{StatementVisitor, walk_body},
    token::TokenKind,
};
use ruff_python_trivia::indentation_at_offset;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        aligner,
        edit::{apply_inline_edits, narrowed_replacement, whole_line_deletion},
        imports::is_import,
        layout::pack,
        slots::runs_where,
    },
    rule::{Rule, RuleId},
    rules::align_imports,
    source::Source,
};

/// Display width of the `import ` keyword and its trailing space, the
/// distance from an aligned `import` column to the first name.
const IMPORT_KEYWORD_WIDTH: usize = "import ".len();

/// What joins two members sharing one line, written between them and
/// counted against the budget each line packs to.
const MEMBER_SEPARATOR: &str = ", ";

/// What distinguishes one `from`-import's module from another, the
/// leading-dot count alongside the module name.
type ModuleKey<'a> = (u32, Option<&'a str>);

pub(crate) struct ReflowImports {
    align_settings: Option<aligner::Settings>,
    divided: Option<Vec<String>>,
    import_line_length: usize,
    merge_members: bool,
    sort_members: bool,
    split_multi_module: bool,
}

impl ReflowImports {
    pub(crate) const MESSAGE: &'static str = "lay out an import block one module per line";

    pub(crate) fn from_config(config: &Config) -> Self {
        let align = &config.rules.align_imports;
        let rules = &config.rules.reflow_imports;
        Self {
            // Reserve the aligned prefix only when `align-imports` runs,
            // carrying its `max-shift` but no line cap so a to-be-split
            // import reads the column it aligns to once split.
            align_settings: align.enabled.then(|| aligner::Settings::from(align)),
            divided: (config.group_imports_enabled() && config.rules.space_statements.enabled)
                .then(|| config.first_party()),
            import_line_length: config.import_width(),
            merge_members: rules.merge_members,
            sort_members: config.alphabetize_siblings_enabled(),
            split_multi_module: rules.split_multi_module,
        }
    }
}

impl Rule for ReflowImports {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let columns = self.align_settings.map_or_else(HashMap::new, |settings| {
            align_imports::aligned_import_columns(source, settings, self.divided.as_deref())
        });
        let mut visitor = Layout {
            columns,
            groups: Vec::new(),
            import_line_length: self.import_line_length,
            merge_members: self.merge_members,
            newline: source.newline_str(),
            sort_members: self.sort_members,
            source,
            split_multi_module: self.split_multi_module,
        };
        visitor.visit_body(&source.ast().body);
        visitor.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Layout<'a> {
    columns: HashMap<TextSize, usize>,
    groups: Vec<Vec<Edit>>,
    import_line_length: usize,
    merge_members: bool,
    newline: &'static str,
    sort_members: bool,
    source: &'a Source,
    split_multi_module: bool,
}

impl<'a> Layout<'a> {
    /// Folds every member of `group` into its first statement, laying
    /// the gathered roster out under the shared prefix and clearing each
    /// folded member's line. A group whose members already read that way
    /// emits nothing.
    fn merge_group(&mut self, body: &'a [Stmt], group: &[usize]) {
        let [lead, .., last] = group else {
            unreachable!("invariant: a merge group holds two or more members");
        };
        let node = body[*lead]
            .as_import_from_stmt()
            .expect("a merge group holds `from`-imports alone");
        let aliases = group
            .iter()
            .filter_map(|&slot| body[slot].as_import_from_stmt())
            .flat_map(|member| &member.names);
        let names = self.roster(aliases);
        let prefix = import_prefix(self.source, node);
        let mut edits: Vec<Edit> = self
            .packed_edit(node, &prefix, &names)
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

    /// Emits the packed rewrite of `node` when its canonical
    /// single-line form overflows the import budget.
    fn pack_lone(&mut self, node: &'a StmtImportFrom) {
        let [_, _, ..] = node.names.as_slice() else {
            return;
        };
        let names = self.roster(node.names.iter());
        let names_width: usize = names.iter().map(|name| name.width()).sum();
        let prefix = import_prefix(self.source, node);
        if self.prefix_width(node, &prefix)
            + names_width
            + MEMBER_SEPARATOR.len() * (names.len() - 1)
            <= self.import_line_length
        {
            return;
        }
        self.groups.extend(
            self.packed_edit(node, &prefix, &names)
                .map(|edit| vec![edit]),
        );
    }

    /// The edit rewriting `node` to carry `names` under `prefix`,
    /// repeated on each line and packed from the prefix column up to the
    /// import budget. `None` when the statement does not open its own
    /// line or already reads that way.
    fn packed_edit(&self, node: &StmtImportFrom, prefix: &str, names: &[&str]) -> Option<Edit> {
        let indent = own_line_indent(self.source, node)?;
        let prefix_width = self.prefix_width(node, prefix);
        let widths: Vec<usize> = names.iter().map(|name| name.width()).collect();
        let joiner = format!("{}{indent}", self.newline);
        let rewrite = pack(
            &widths,
            prefix_width,
            MEMBER_SEPARATOR.len(),
            self.import_line_length,
        )
        .into_iter()
        .map(|range| format!("{prefix}{}", names[range].join(MEMBER_SEPARATOR)))
        .join(&joiner);
        narrowed_replacement(self.source, node.range(), rewrite)
    }

    /// Display width the first name lands at, the aligned `import`
    /// column when `align-imports` pads the keyword rightward and the
    /// natural prefix width otherwise.
    fn prefix_width(&self, node: &StmtImportFrom, prefix: &str) -> usize {
        self.columns.get(&node.start()).map_or_else(
            || self.source.line_indent_width(node.start()) + prefix.width(),
            |&column| column + IMPORT_KEYWORD_WIDTH,
        )
    }

    /// Folds each repeated module in `body` into one statement and
    /// splits every comma-joined bare import, one fix group apiece.
    fn process_body(&mut self, body: &'a [Stmt]) {
        let groups = if self.merge_members {
            module_groups(self.source, body)
        } else {
            Vec::new()
        };
        let gathered: Vec<usize> = groups.iter().flatten().copied().collect();
        for group in &groups {
            self.merge_group(body, group);
        }
        for (slot, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Import(bare) if self.split_multi_module => self.split_bare_import(bare),
                Stmt::ImportFrom(lone) if !gathered.contains(&slot) => self.pack_lone(lone),
                _ => {}
            }
        }
    }

    /// The de-duplicated source text of `aliases`, the member roster one
    /// module's line carries, ordered as `alphabetize-siblings` would leave it
    /// unless that rule is off.
    fn roster(&self, aliases: impl Iterator<Item = &'a Alias>) -> Vec<&'a str> {
        let mut names: Vec<&str> = aliases
            .map(|alias| self.source.slice(alias.range()))
            .unique()
            .collect();
        if self.sort_members {
            names.sort_unstable();
        }
        names
    }

    /// Emits the one-statement-per-module rewrite of a comma-joined
    /// bare import.
    fn split_bare_import(&mut self, node: &StmtImport) {
        let [_, _, ..] = node.names.as_slice() else {
            return;
        };
        let Some(indent) = own_line_indent(self.source, node) else {
            return;
        };
        let joiner = format!("{}{indent}", self.newline);
        let rewrite = node
            .names
            .iter()
            .map(|alias| format!("import {}", self.source.slice(alias.range())))
            .join(&joiner);
        self.groups.extend(
            narrowed_replacement(self.source, node.range(), rewrite).map(|edit| vec![edit]),
        );
    }
}

impl<'a> StatementVisitor<'a> for Layout<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.process_body(body);
        walk_body(self, body);
    }
}

/// True when the members at `slots` fold into one statement without
/// disturbing their surroundings, being two or more statements of one
/// notebook cell with no comment anywhere across the full lines the
/// gather clears.
fn gathers_cleanly(source: &Source, body: &[Stmt], slots: &[usize]) -> bool {
    let [first, .., last] = slots else {
        return false;
    };
    let span =
        source.full_lines_within_cell(TextRange::new(body[*first].start(), body[*last].end()));
    source.same_cell(span.start(), span.end()) && !source.intersects_comment(span)
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

/// Builds the `from <dots><module> import ` prefix each split line
/// repeats, with the relative-import leading dots folded into it and
/// the source's own gap before `import` carried through.
fn import_prefix(source: &Source, node: &StmtImportFrom) -> String {
    format!(
        "from {}{}import ",
        format_import_from(node.level, node.module.as_deref()),
        import_keyword_gap(source, node).unwrap_or(" "),
    )
}

/// True when `node` can join a merged roster, being a single-line
/// `from`-import that opens its own line and binds no star member,
/// since `*` admits no sibling on its statement.
fn mergeable(source: &Source, node: &StmtImportFrom) -> bool {
    own_line_indent(source, node).is_some()
        && !node.names.iter().any(|alias| alias.name.as_str() == "*")
}

/// The same-module merges `reflow-imports` will make, forecast by a
/// rule seated ahead of it whose drop then lands on the merged line
/// rather than on a statement of its own.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Merges {
    enabled: bool,
}

impl Merges {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.reflow_imports;
        Self {
            enabled: rules.enabled && rules.merge_members,
        }
    }

    /// The slot groups of `body` the rule folds together, empty where it
    /// merges nothing.
    pub(crate) fn groups(self, source: &Source, body: &[Stmt]) -> Vec<Vec<usize>> {
        if !self.enabled {
            return Vec::new();
        }
        module_groups(source, body)
    }

    /// True when `groups` folds the statement at `slot` into a sibling
    /// that `survives` the drops the caller makes, so the drop of its
    /// own names lands on the merged line rather than on a statement
    /// of its own.
    pub(crate) fn folds(
        groups: &[Vec<usize>],
        slot: usize,
        survives: impl Fn(usize) -> bool,
    ) -> bool {
        groups
            .iter()
            .filter(|group| group.contains(&slot))
            .any(|group| group.iter().any(|&other| other != slot && survives(other)))
    }
}

/// Slot groups of two or more mergeable `from`-imports sharing one
/// module within a run of adjacent import statements, each group
/// spanning one notebook cell and carrying no comment between its
/// first and last member.
fn module_groups(source: &Source, body: &[Stmt]) -> Vec<Vec<usize>> {
    let mut groups = Vec::new();
    for run in runs_where(body, is_import) {
        let mut by_module: Vec<(ModuleKey, Vec<usize>)> = Vec::new();
        for slot in run {
            let Some(node) = body[slot]
                .as_import_from_stmt()
                .filter(|n| mergeable(source, n))
            else {
                continue;
            };
            let key = module_key(node);
            match by_module.iter_mut().find(|(seen, _)| *seen == key) {
                Some((_, slots)) => slots.push(slot),
                None => by_module.push((key, vec![slot])),
            }
        }
        groups.extend(
            by_module
                .into_iter()
                .map(|(_, slots)| slots)
                .filter(|slots| gathers_cleanly(source, body, slots)),
        );
    }
    groups
}

/// The module `node` imports from.
fn module_key(node: &StmtImportFrom) -> ModuleKey<'_> {
    (node.level, node.module.as_deref())
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

    #[rstest]
    #[case("from a.b.c import x\n", "from a.b.c import ")]
    #[case("from . import x\n", "from . import ")]
    #[case("from .sub import x\n", "from .sub import ")]
    #[case("from ..pkg import x\n", "from ..pkg import ")]
    #[case("from typing     import x\n", "from typing     import ")]
    fn import_prefix_folds_relative_dots_into_the_repeated_prefix(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let node = source.ast().body[0]
            .as_import_from_stmt()
            .expect("first statement is a from-import");
        assert_eq!(import_prefix(&source, node), expected);
    }

    #[test]
    fn multi_line_import_is_left_untouched() {
        let source = parse("from pkg import (\n    alpha,\n    beta,\n    gamma,\n)\n");
        let rule = ReflowImports {
            align_settings: None,
            divided: None,
            import_line_length: 10,
            merge_members: true,
            sort_members: true,
            split_multi_module: true,
        };

        assert!(rule.apply(&source).is_empty());
    }

    #[test]
    fn semicolon_joined_bare_import_is_left_untouched() {
        let source = parse("x = 1; import os, sys\n");
        let rule = ReflowImports {
            align_settings: None,
            divided: None,
            import_line_length: 10,
            merge_members: true,
            sort_members: true,
            split_multi_module: true,
        };

        assert!(rule.apply(&source).is_empty());
    }

    #[test]
    fn semicolon_joined_import_is_left_untouched() {
        let source = parse("x = 1; from pkg import alpha, beta, gamma\n");
        let rule = ReflowImports {
            align_settings: None,
            divided: None,
            import_line_length: 10,
            merge_members: true,
            sort_members: true,
            split_multi_module: true,
        };

        assert!(rule.apply(&source).is_empty());
    }
}
