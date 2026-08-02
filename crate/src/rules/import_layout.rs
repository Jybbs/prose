//! Splits an over-budget `from <module> import …` into repeated-prefix
//! lines, greedily packing the names up to `Config::import_line_length`,
//! or up to the aligned prefix when `align-imports` pads the `import`
//! keyword rightward. A single-name, within-budget, or multi-line import
//! stays untouched, and a lone name whose own line overflows keeps it
//! rather than splitting further.

use std::{collections::HashMap, ops::Range};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Stmt, StmtImportFrom,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_python_trivia::indentation_at_offset;
use ruff_text_size::{Ranged, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        aligner,
        edit::{narrowed_replacement, singleton_groups},
    },
    rule::{Rule, RuleId},
    rules::align_imports,
    source::Source,
};

/// Display width of the `import ` keyword and its trailing space, the
/// distance from an aligned `import` column to the first name.
const IMPORT_KEYWORD_WIDTH: usize = "import ".len();

pub(crate) struct ImportLayout {
    align_settings: Option<aligner::Settings>,
    import_line_length: usize,
}

impl ImportLayout {
    pub(crate) const MESSAGE: &'static str =
        "split an over-long `from` import into repeated-prefix lines";

    pub(crate) fn from_config(config: &Config) -> Self {
        let align = &config.rules.align_imports;
        Self {
            // Reserve the aligned prefix only when `align-imports` runs,
            // carrying its `max-shift` but no line cap so a to-be-split
            // import reads the column it aligns to once split.
            align_settings: align.enabled.then(|| aligner::Settings::from(align)),
            import_line_length: config.import_width(),
        }
    }
}

impl Rule for ImportLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let columns = self.align_settings.map_or_else(HashMap::new, |settings| {
            align_imports::aligned_import_columns(source, settings)
        });
        let mut visitor = Layout {
            columns,
            edits: Vec::new(),
            import_line_length: self.import_line_length,
            newline: source.newline_str(),
            source,
        };
        visitor.visit_body(&source.ast().body);
        singleton_groups(visitor.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Layout<'a> {
    columns: HashMap<TextSize, usize>,
    edits: Vec<Edit>,
    import_line_length: usize,
    newline: &'static str,
    source: &'a Source,
}

impl<'a> Layout<'a> {
    /// Returns the leading-whitespace prefix of `node`'s line when
    /// `node` begins that line, or `None` when other code precedes it
    /// (a `;`-joined simple statement), where splitting would strand a
    /// continuation line at the wrong indent.
    fn line_indent(&self, node: &StmtImportFrom) -> Option<&'a str> {
        indentation_at_offset(node.start(), self.source.text())
    }

    /// Emits the packed multi-line rewrite of `node` when its canonical
    /// single-line form overflows the import budget.
    fn process_import(&mut self, node: &StmtImportFrom) {
        if self.source.contains_line_break(node.range()) {
            return;
        }
        let [_, _, ..] = node.names.as_slice() else {
            return;
        };
        let Some(indent) = self.line_indent(node) else {
            return;
        };
        let prefix = import_prefix(node);
        let names: Vec<&str> = node
            .names
            .iter()
            .map(|alias| self.source.slice(alias.range()))
            .collect();
        let widths: Vec<usize> = names.iter().map(|name| name.width()).collect();
        // Pack against the aligned prefix when `align-imports` will pad the
        // `import` keyword rightward, so the names still clear the budget
        // once the padding lands. An unaligned import reads its natural
        // prefix width.
        let prefix_width = self.columns.get(&node.start()).map_or_else(
            || indent.chars().count() + prefix.width(),
            |&column| column + IMPORT_KEYWORD_WIDTH,
        );
        let single_line = prefix_width + widths.iter().sum::<usize>() + 2 * (widths.len() - 1);
        if single_line <= self.import_line_length {
            return;
        }
        let joiner = format!("{}{indent}", self.newline);
        let rewrite = pack(&widths, prefix_width, self.import_line_length)
            .into_iter()
            .map(|range| format!("{prefix}{}", names[range].join(", ")))
            .collect::<Vec<_>>()
            .join(&joiner);
        self.edits
            .extend(narrowed_replacement(self.source, node.range(), rewrite));
    }
}

impl<'a> StatementVisitor<'a> for Layout<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::ImportFrom(node) = stmt {
            self.process_import(node);
        }
        walk_stmt(self, stmt);
    }
}

/// Builds the `from <dots><module> import ` prefix each split line
/// repeats, with the relative-import leading dots folded into it.
fn import_prefix(node: &StmtImportFrom) -> String {
    format!(
        "from {dots}{module} import ",
        dots = ".".repeat(node.level as usize),
        module = node.module.as_deref().unwrap_or(""),
    )
}

/// Greedily groups name indices into lines, each opening after the
/// shared `prefix_width` and packing names (joined by `", "`) up to
/// `budget`. The first name on every line is always placed, so a name
/// whose own line overflows still lands rather than splitting away.
fn pack(widths: &[usize], prefix_width: usize, budget: usize) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut line_width = 0;
    for (i, &width) in widths.iter().enumerate() {
        if i == start {
            line_width = prefix_width + width;
        } else if line_width + 2 + width <= budget {
            line_width += 2 + width;
        } else {
            lines.push(start..i);
            start = i;
            line_width = prefix_width + width;
        }
    }
    lines.push(start..widths.len());
    lines
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case("from a.b.c import x\n", "from a.b.c import ")]
    #[case("from . import x\n", "from . import ")]
    #[case("from .sub import x\n", "from .sub import ")]
    #[case("from ..pkg import x\n", "from ..pkg import ")]
    fn import_prefix_folds_relative_dots_into_the_repeated_prefix(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let node = source.ast().body[0]
            .as_import_from_stmt()
            .expect("first statement is a from-import");
        assert_eq!(import_prefix(node), expected);
    }

    #[test]
    fn multi_line_import_is_left_untouched() {
        let source = parse("from pkg import (\n    alpha,\n    beta,\n    gamma,\n)\n");
        let rule = ImportLayout {
            align_settings: None,
            import_line_length: 10,
        };
        assert!(rule.apply(&source).is_empty());
    }

    #[test]
    fn pack_carries_a_lone_overflowing_name_onto_its_own_line() {
        // prefix 10, budget 14, name widths 8/8: neither pairs onto a
        // line, so each forced name lands alone despite overflowing.
        assert_eq!(pack(&[8, 8], 10, 14), vec![0..1, 1..2]);
    }

    #[test]
    fn pack_fills_each_line_before_opening_the_next() {
        // prefix 5, budget 16: 4 then 4 (5+4=9, +2+4=15) fit, 4 more
        // (15+2+4=21) overflows and opens a line that then takes the 4.
        assert_eq!(pack(&[4, 4, 4], 5, 16), vec![0..2, 2..3]);
    }

    #[test]
    fn pack_keeps_one_line_when_every_name_fits() {
        assert_eq!(pack(&[1, 1, 1], 5, 80), vec![0..3]);
    }

    #[test]
    fn semicolon_joined_import_is_left_untouched() {
        let source = parse("x = 1; from pkg import alpha, beta, gamma\n");
        let rule = ImportLayout {
            align_settings: None,
            import_line_length: 10,
        };
        assert!(rule.apply(&source).is_empty());
    }
}
