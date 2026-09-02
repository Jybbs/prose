//! Helpers shared across `#[cfg(test)] mod tests` blocks.

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use ruff_diagnostics::{Edit, SourceMap};
use ruff_notebook::{Notebook, NotebookIndex};
use ruff_python_ast::{Expr, PySourceType, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_text_size::{TextLen, TextRange, TextSize};
use serde_json::{Value, json};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    pipeline::Pipeline,
    primitives::{
        aligner,
        edit::apply_edits_mapped,
        tiering::{Evaluated, call_reachable, eval_time_refs_of},
    },
    rule::{Rule, RuleId},
    source::Source,
    walker::{Found, walk},
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
#[derive(Debug)]
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

/// A rule emitting `edit` only while the buffer's text opens with
/// `guard`. An `edit` that keeps the opening matching `guard` edits its
/// own output forever, and one that breaks the match settles after a
/// single edit.
#[derive(Debug)]
pub(crate) struct GuardedRule {
    pub(crate) edit: Edit,
    pub(crate) guard: &'static str,
    pub(crate) id: RuleId,
}

impl Rule for GuardedRule {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        if source.text().starts_with(self.guard) {
            vec![vec![self.edit.clone()]]
        } else {
            Vec::new()
        }
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn message(&self) -> &'static str {
        "guarded test rule"
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
        baseline: (gap.start() - TextSize::new(line_start)).to_usize() - width,
        gap,
        line_start: TextSize::new(line_start),
        op_width: 0,
        settled_width: width,
        value_gap: None,
        width,
    }
}

pub(crate) fn applied_text(source: &Source, edits: Vec<Edit>) -> String {
    woven(source.text(), edits).0
}

pub(crate) fn assert_send_sync<T: Send + Sync>() {}

/// The range of the first `needle` in `text`.
pub(crate) fn at(text: &str, needle: &str) -> TextRange {
    let start = TextSize::try_from(text.find(needle).expect("the needle is present"))
        .expect("the offset fits");
    TextRange::at(start, needle.text_len())
}

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

/// Every Python module and notebook under the tree
/// `PROSE_SETTLE_CORPUS` names, the fixture tree absent it, ascending
/// by path.
pub(crate) fn corpus_inputs() -> Vec<PathBuf> {
    let root = std::env::var_os("PROSE_SETTLE_CORPUS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        PathBuf::from,
    );
    let mut inputs: Vec<PathBuf> = ignore::WalkBuilder::new(root)
        .standard_filters(false)
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.is_file() && PySourceType::try_from_path(path).is_some())
        .collect();
    inputs.sort();
    inputs
}

/// `body`'s evaluated pair under its full call reach, annotations
/// eager.
pub(crate) fn evaluated<'src>(source: &'src Source, body: &'src [Stmt]) -> Evaluated<'src> {
    Evaluated::of(
        body,
        &call_reachable(source.binding_analysis(), body),
        eval_time_refs_of(body, false),
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

/// The formattable paths under `paths`, a walk error or a passed link
/// failing the test outright.
pub(crate) fn formattable(paths: &[PathBuf]) -> Vec<PathBuf> {
    walk(paths)
        .filter_map(|found| match found.expect("the tree walks") {
            Found::Formattable(path, _) => Some(path),
            Found::PassedLink(_) => None,
        })
        .collect()
}

/// A rule replacing the source's first byte with `yy`, so every pass
/// over its own output grows the line and edits again.
pub(crate) fn never_settles(id: &'static str) -> GroupSentinelRule {
    GroupSentinelRule {
        groups: vec![vec![replacement("yy", 0, 1)]],
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

/// A [`GuardedRule`] replacing the buffer's first byte with `writes`
/// while its text opens with `reads`.
pub(crate) fn prefix_rule(
    id: &'static str,
    reads: &'static str,
    writes: &'static str,
) -> GuardedRule {
    GuardedRule {
        edit: replacement(writes, 0, 1),
        guard: reads,
        id: RuleId::from(id),
    }
}

pub(crate) fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(start.into(), end.into())
}

/// An edit replacing the `start..end` span with `content`.
pub(crate) fn replacement(content: &str, start: u32, end: u32) -> Edit {
    Edit::range_replacement(content.to_owned(), range(start, end))
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
        groups: vec![vec![replacement("Y", 0, 3), replacement("Z", 2, 5)]],
        id: RuleId::from("self-overlapping"),
    }
}

/// The text `edits` weave into `text`, beside the `SourceMap` of the
/// weave.
pub(crate) fn woven(text: &str, edits: Vec<Edit>) -> (String, SourceMap) {
    apply_edits_mapped(text, edits).expect("the edits weave")
}

pub(crate) fn write_dotconfig_prose_toml(dir: &Path, contents: &str) {
    write_config(&dir.join(".config"), "prose.toml", contents);
}

pub(crate) fn write_prose_toml(dir: &Path, contents: &str) {
    write_config(dir, "prose.toml", contents);
}

pub(crate) fn write_pyproject(dir: &Path, contents: &str) {
    write_config(dir, "pyproject.toml", contents);
}

/// Writes `contents` as the config file `name` under `dir`, creating
/// the directory where it is absent.
fn write_config(dir: &Path, name: &str, contents: &str) {
    fs_err::create_dir_all(dir).expect("the config directory creates");
    fs_err::write(dir.join(name), contents).expect("the config writes");
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
