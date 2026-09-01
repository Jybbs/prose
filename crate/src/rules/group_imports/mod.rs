//! Partitions a module's imports into the canonical sections
//! `__future__` → bare → external `from` → local-package, relocating
//! each contiguous import run within a section into group order while
//! leaving the names within a group to `alphabetize-siblings`. The
//! first-party list under `[imports]` decides local-package membership.

use std::{borrow::Cow, ops::Range};

use ruff_diagnostics::Edit;
use ruff_python_ast::Stmt;
use ruff_text_size::TextRange;

use crate::{
    config::Config,
    primitives::{
        edit::{narrowed_replacement, singleton_groups},
        imports::{import_group, sectioned_import_runs},
        orderer::{any_sibling_shares_line, assemble_blocks, member_blocks, permute_full},
        range::blocks_span,
        scope::sub_bodies,
        sections::Sections,
    },
    rule::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct GroupImports {
    first_party: Vec<String>,
}

impl GroupImports {
    pub(crate) const MESSAGE: &'static str =
        "group imports into bare, external, and local sections";

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
    /// next to `space-statements`.
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
        let newline = self.source.newline_str();
        let assembled = assemble_blocks(self.source, run_blocks, &rendered, &order, |_| {
            Some(newline)
        });
        if let Some(edit) = narrowed_replacement(self.source, blocks_span(run_blocks), assembled) {
            self.edits.push(edit);
        }
    }
}
