//! Helpers shared across `#[cfg(test)] mod tests` blocks.

use std::{
    io::{self, Write},
    path::Path,
};

use ruff_diagnostics::{Edit, SourceMap};
use ruff_notebook::{Notebook, NotebookIndex};
use ruff_python_ast::{Expr, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_text_size::{TextLen, TextRange, TextSize};
use serde_json::{Value, json};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    pipeline::Pipeline,
    primitives::{
        aligner,
        edit::apply_edits_mapped,
        tiering::{Evaluated, call_reachable},
    },
    rule::{Rule, RuleId},
    source::Source,
};

/// Formatted module source whose bare `import os` draws one
/// `bare-imports` finding and no format edit.
pub(crate) const BARE_IMPORT_LINT: &str = "import os\n\nos.getcwd()\n";

/// Module source leading with a `__future__` import, the input
/// [`breaks_compile`] demotes.
pub(crate) const FUTURE_LEAD: &str = "from __future__ import annotations\nimport os\n";

/// Test-only writer whose `write` fails with the supplied kind.
pub(crate) struct FailingWriter(pub(crate) io::ErrorKind);

impl Write for FailingWriter {
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(self.0.into())
    }
}

/// Test-only rule that returns the fix groups supplied at
/// construction.
pub(crate) struct GroupSentinelRule {
    pub(crate) groups: Vec<Vec<Edit>>,
    pub(crate) id: RuleId,
}

impl Rule for GroupSentinelRule {
    fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
        self.groups.clone()
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn message(&self) -> &'static str {
        "group test rule"
    }

    fn preserves_bindings(&self) -> bool {
        false
    }
}

/// A rule editing only a buffer whose text opens with `reads`,
/// replacing that buffer's first byte with `writes`. A `writes` that
/// keeps the opening matching `reads` edits its own output forever,
/// and one that breaks the match settles after a single edit.
pub(crate) struct PrefixRule {
    pub(crate) id: RuleId,
    pub(crate) reads: &'static str,
    pub(crate) writes: &'static str,
}

impl Rule for PrefixRule {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        if source.text().starts_with(self.reads) {
            vec![vec![Edit::range_replacement(
                self.writes.to_owned(),
                range(0, 1),
            )]]
        } else {
            Vec::new()
        }
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn message(&self) -> &'static str {
        "prefix test rule"
    }

    fn preserves_bindings(&self) -> bool {
        false
    }
}

/// Builds an alignment `Member` whose pre-operator whitespace is `gap`,
/// carrying no operator width and no post-operator gap, its baseline
/// read off the ASCII columns between `line_start` and the gap. Layer
/// `with_op_width`, `with_settled_width`, or `with_value_gap` on top
/// for a row that needs one.
pub(crate) fn align_member(gap: TextRange, line_start: u32, width: usize) -> aligner::Member {
    aligner::Member {
        baseline: (gap.start() - TextSize::new(line_start))
            .to_usize()
            .saturating_sub(width),
        gap,
        line_start: TextSize::new(line_start),
        op_width: 0,
        settled_width: width,
        value_gap: None,
        width,
    }
}

pub(crate) fn applied_text(source: &Source, edits: Vec<Edit>) -> String {
    apply_edits_mapped(source.text(), edits)
        .expect("non-overlapping edits")
        .0
}

pub(crate) fn assert_send_sync<T: Send + Sync>() {}

/// Returns a rule whose single edit demotes a leading `__future__`
/// import below the import after it, output that parses and no longer
/// compiles. Pair it with [`FUTURE_LEAD`] as the source.
pub(crate) fn breaks_compile() -> GroupSentinelRule {
    GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement(
            "import os\nfrom __future__ import annotations".to_owned(),
            TextRange::up_to(FUTURE_LEAD.trim_end().text_len()),
        )]],
        id: RuleId::from("breaks-compile"),
    }
}

/// Returns a rule whose single edit rewrites the leading statement
/// into unparseable source.
pub(crate) fn breaks_parse() -> GroupSentinelRule {
    GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement(
            "def foo(".to_owned(),
            range(0, 5),
        )]],
        id: RuleId::from("breaks-parse"),
    }
}

/// `body`'s evaluated pair under its full call reach, annotations
/// eager.
pub(crate) fn evaluated<'src>(source: &'src Source, body: &'src [Stmt]) -> Evaluated<'src> {
    Evaluated::of(
        body,
        &call_reachable(source.binding_analysis(), body),
        false,
    )
}

pub(crate) fn first_class(source: &Source) -> &StmtClassDef {
    source.ast().body[0]
        .as_class_def_stmt()
        .expect("first statement is a class")
}

pub(crate) fn first_def(source: &Source) -> &StmtFunctionDef {
    source.ast().body[0]
        .as_function_def_stmt()
        .expect("first statement is a def")
}

pub(crate) fn first_expr(source: &Source) -> &Expr {
    &source.ast().body[0]
        .as_expr_stmt()
        .expect("first statement is an expression")
        .value
}

pub(crate) fn first_value(source: &Source) -> &Expr {
    &source.ast().body[0]
        .as_assign_stmt()
        .expect("first statement is an assignment")
        .value
}

/// Format diagnostic with a safe single-edit fix.
pub(crate) fn format_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::format(
        RuleId::from("rewrite-x"),
        vec![Edit::range_replacement("y".to_owned(), range)],
        "rewrite x to y".to_owned(),
    )
}

/// A rule replacing the source's first byte with `yy`, so every pass
/// over its own output grows the line and edits again.
pub(crate) fn never_settles(id: &'static str) -> GroupSentinelRule {
    GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement("yy".to_owned(), range(0, 1))]],
        id: RuleId::from(id),
    }
}

/// Builds a notebook-backed `Source` from per-cell Python sources, the
/// `Ipynb` counterpart to [`parse`]. The cells concatenate through the
/// synthetic separator `ruff_notebook` inserts, so the returned source
/// carries real cell boundaries.
pub(crate) fn notebook(cells: &[&str]) -> Source {
    Source::from_notebook(&notebook_document(cells), "<nb>").expect("notebook source builds")
}

/// The cell index of a notebook built from `cells`, the translator a
/// report renders each concatenated position through.
pub(crate) fn notebook_index(cells: &[&str]) -> NotebookIndex {
    notebook_document(cells).into_index()
}

pub(crate) fn parse(src: &str) -> Source {
    src.parse().expect("test source parses")
}

pub(crate) fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(start.into(), end.into())
}

/// A rule replacing the source's first byte with `y`, the one-edit
/// rewrite most pipeline tests run.
pub(crate) fn rewrites_x_to_y() -> GroupSentinelRule {
    GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
        id: RuleId::from("rewrite-x-to-y"),
    }
}

pub(crate) fn run_rule(slug: &str, src: &str) -> String {
    let pipeline = Pipeline::for_rule(slug, &Config::default()).expect("rule is registered");
    pipeline
        .run(parse(src))
        .expect("pipeline runs")
        .0
        .text()
        .to_owned()
}

/// Returns a rule whose single group holds two edits over overlapping
/// ranges, a group the splice declines to apply.
pub(crate) fn self_overlapping() -> GroupSentinelRule {
    GroupSentinelRule {
        groups: vec![vec![
            Edit::range_replacement("Y".to_owned(), range(0, 3)),
            Edit::range_replacement("Z".to_owned(), range(2, 5)),
        ]],
        id: RuleId::from("self-overlapping"),
    }
}

/// The text `edits` weave into `text`, beside the `SourceMap` of the
/// weave.
pub(crate) fn woven(text: &str, edits: Vec<Edit>) -> (String, SourceMap) {
    apply_edits_mapped(text, edits).expect("the edits weave")
}

pub(crate) fn write_dotconfig_prose_toml(dir: &Path, contents: &str) {
    let config_dir = dir.join(".config");
    std::fs::create_dir_all(&config_dir).expect(".config dir creates");
    std::fs::write(config_dir.join("prose.toml"), contents).expect(".config/prose.toml writes");
}

pub(crate) fn write_prose_toml(dir: &Path, contents: &str) {
    std::fs::write(dir.join("prose.toml"), contents).expect("prose.toml writes");
}

pub(crate) fn write_pyproject(dir: &Path, contents: &str) {
    std::fs::write(dir.join("pyproject.toml"), contents).expect("pyproject writes");
}

/// The parsed notebook `cells` describes, one code cell per source.
fn notebook_document(cells: &[&str]) -> Notebook {
    let cells: Vec<Value> = cells
        .iter()
        .map(|source| {
            json!({
                "cell_type": "code",
                "execution_count": null,
                "metadata": {},
                "outputs": [],
                "source": source,
            })
        })
        .collect();
    let document = json!({
        "cells": cells,
        "metadata": { "language_info": { "name": "python" } },
        "nbformat": 4,
        "nbformat_minor": 5,
    });
    let json = serde_json::to_string(&document).expect("notebook json serializes");
    Notebook::from_source_code(&json).expect("notebook parses")
}
