//! Partitions a module's imports into the canonical sections
//! bare → external `from` → local-package, relocating each contiguous
//! import run within a section into group order while leaving the names
//! within a group to `alphabetize`. The first-party list under
//! `[imports]` decides local-package membership.

use std::{borrow::Cow, ops::Range};

use ruff_diagnostics::Edit;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        edit::{narrowed_replacement, singleton_groups},
        imports::{import_group, sectioned_import_runs},
        orderer::{
            any_sibling_shares_line, assemble_blocks, blocks_span, member_blocks, permute_full,
        },
        scope::{compound_sub_bodies, scoped_body},
        sections::Sections,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct GroupImports {
    first_party: Vec<String>,
}

impl GroupImports {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            first_party: config.first_party(),
        }
    }
}

impl Rule for GroupImports {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut walker = Walker {
            edits: Vec::new(),
            first_party: &self.first_party,
            source,
        };
        walker.group_body(&source.ast().body, source.module_range());
        singleton_groups(walker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Walker<'a> {
    edits: Vec<Edit>,
    first_party: &'a [String],
    source: &'a Source,
}

impl Walker<'_> {
    /// Partitions each import run in `body`, then recurses into every
    /// nested body. A run reorders within one section, and a body whose
    /// siblings share a physical line through `;` keeps source order.
    fn group_body(&mut self, body: &[Stmt], outer: TextRange) {
        if !body.is_empty() && !any_sibling_shares_line(self.source, body) {
            let blocks = member_blocks(self.source, body, outer);
            let sections = Sections::of(self.source, &blocks);
            for run in sectioned_import_runs(&sections, body) {
                self.group_run(body, &blocks, run);
            }
        }
        for stmt in body {
            for (sub, sub_outer) in sub_bodies(stmt) {
                self.group_body(sub, sub_outer);
            }
        }
    }

    /// Relocates the imports in `run` into canonical group order, the
    /// names within a group left in place. Emits one edit only when the
    /// partition rewrites the order, seating every import tight against
    /// its neighbor and leaving the blank dividing one section from the
    /// next to `blank-lines`.
    fn group_run(&mut self, body: &[Stmt], blocks: &[TextRange], run: Range<usize>) {
        let items = &body[run.clone()];
        let mut order: Vec<usize> = (0..items.len()).collect();
        if !permute_full(&mut order, items, |stmt| {
            import_group(stmt, self.first_party)
        }) {
            return;
        }
        let run_blocks = &blocks[run];
        let rendered: Vec<Cow<'_, str>> = run_blocks
            .iter()
            .map(|&block| Cow::Borrowed(self.source.slice(block)))
            .collect();
        let assembled = assemble_blocks(self.source, run_blocks, &rendered, &order, |_| Some("\n"));
        if let Some(edit) = narrowed_replacement(self.source, blocks_span(run_blocks), assembled) {
            self.edits.push(edit);
        }
    }
}

/// Returns the body and enclosing range of every direct sub-body a
/// statement opens, the class- or function-definition suite and each arm
/// of a compound statement alike. Empty sub-bodies are returned as-is and
/// skipped by the caller.
fn sub_bodies(stmt: &Stmt) -> Vec<(&[Stmt], TextRange)> {
    if let Some((body, _)) = scoped_body(stmt) {
        return vec![(body, stmt.range())];
    }
    compound_sub_bodies(stmt)
}
