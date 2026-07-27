//! Helpers shared across `#[cfg(test)] mod tests` blocks.

use std::{
    io::{self, Write},
    path::Path,
};

use ruff_diagnostics::Edit;
use ruff_notebook::Notebook;
use ruff_python_ast::{Expr, StmtClassDef, StmtFunctionDef};
use ruff_text_size::{TextLen, TextRange, TextSize};
use serde_json::{Value, json};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    pipeline::Pipeline,
    primitives::{aligner, edit::apply_edits},
    rule::{Rule, RuleId},
    source::Source,
};

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
}

/// Builds an alignment `Member` whose pre-operator whitespace is `gap`,
/// carrying no operator width and no post-operator gap. Layer
/// `with_op_width` or `with_value_gap` on top for a row that needs
/// either.
pub(crate) fn align_member(gap: TextRange, line_start: u32, width: usize) -> aligner::Member {
    aligner::Member {
        gap,
        line_start: TextSize::new(line_start),
        op_width: 0,
        value_gap: None,
        width,
    }
}

pub(crate) fn applied_text(source: &Source, edits: Vec<Edit>) -> String {
    apply_edits(source.text(), edits).expect("non-overlapping edits")
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

/// Builds a notebook-backed `Source` from per-cell Python sources, the
/// `Ipynb` counterpart to [`parse`]. The cells concatenate through the
/// synthetic separator `ruff_notebook` inserts, so the returned source
/// carries real cell boundaries.
pub(crate) fn notebook(cells: &[&str]) -> Source {
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
    let parsed = Notebook::from_source_code(&json).expect("notebook parses");
    Source::from_notebook(&parsed, "<nb>").expect("notebook source builds")
}

pub(crate) fn parse(src: &str) -> Source {
    src.parse().expect("test source parses")
}

pub(crate) fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(start.into(), end.into())
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
