//! Helpers shared across `#[cfg(test)] mod tests` blocks.

use std::{path::Path, str::FromStr};

use lsp_types::Uri;
use ruff_diagnostics::Edit;
use ruff_python_ast::{StmtClassDef, StmtFunctionDef};
use ruff_text_size::TextRange;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    pipeline::Pipeline,
    primitives::edit::apply_edits,
    rule::{Rule, RuleId},
    source::Source,
};

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

pub(crate) fn applied_text(source: &Source, edits: Vec<Edit>) -> String {
    apply_edits(source.text(), edits).expect("non-overlapping edits")
}

pub(crate) fn assert_send_sync<T: Send + Sync>() {}

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

/// Format diagnostic with a safe single-edit fix.
pub(crate) fn format_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::format(
        RuleId::from("rewrite-x"),
        vec![Edit::range_replacement("y".to_owned(), range)],
        "rewrite x to y".to_owned(),
    )
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

pub(crate) fn uri(s: &str) -> Uri {
    Uri::from_str(s).expect("valid uri")
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
