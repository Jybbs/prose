//! The names a module binds, meaning the rows of the first module-level
//! statement binding each name, walked into compound statements and not into
//! a function, class, comprehension, or lambda.

use std::{collections::BTreeMap, ops::Range, slice::from_ref};

use ruff_python_ast::{
    Expr, ExprContext, Stmt,
    helpers::any_over_expr,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_python_parser::parse_module;
use ruff_source_file::LineIndex;
use ruff_text_size::Ranged;

use crate::format::row_of;

/// The walk recording where each module-level name is first bound.
struct Walk<'a> {
    /// The line index the rows are read through.
    lines: &'a LineIndex,
    /// The rows each name is first bound on.
    rows: BTreeMap<String, Range<usize>>,
}

impl Walk<'_> {
    /// The rows one statement binds its names on, which is the header alone
    /// for a function or class.
    fn header_rows(&self, statement: &Stmt) -> Range<usize> {
        let (head, body) = match statement {
            Stmt::ClassDef(class) => (class.name.range().start(), class.body.first()),
            Stmt::FunctionDef(function) => (function.name.range().start(), function.body.first()),
            _ => (statement.range().start(), None),
        };
        let start = row_of(self.lines, usize::from(head));
        match body {
            Some(first) => {
                start..row_of(self.lines, usize::from(first.range().start())).max(start + 1)
            }
            None => start..row_of(self.lines, usize::from(statement.range().end())) + 1,
        }
    }
}

impl<'a> StatementVisitor<'a> for Walk<'_> {
    fn visit_stmt(&mut self, statement: &'a Stmt) {
        let names = bound(statement);
        if !names.is_empty() {
            let rows = self.header_rows(statement);
            for name in names {
                self.rows.entry(name).or_insert_with(|| rows.clone());
            }
        }
        if !matches!(statement, Stmt::ClassDef(_) | Stmt::FunctionDef(_)) {
            walk_stmt(self, statement);
        }
    }
}

/// The rows of the first module-level statement binding each name, empty
/// where the module does not parse.
pub(crate) fn binding_rows(text: &str) -> BTreeMap<String, Range<usize>> {
    let Ok(parsed) = parse_module(text) else {
        return BTreeMap::new();
    };
    let lines = LineIndex::from_source_text(text);
    let mut walk = Walk {
        lines: &lines,
        rows: BTreeMap::new(),
    };
    walk.visit_body(&parsed.syntax().body);
    walk.rows
}

/// The names one module-level statement binds, which is the name of a
/// definition, the first segment of each import, or every name it stores.
fn bound(statement: &Stmt) -> Vec<String> {
    match statement {
        Stmt::AnnAssign(assign) => stored(from_ref(&assign.target)),
        Stmt::Assign(assign) => stored(&assign.targets),
        Stmt::AugAssign(assign) => stored(from_ref(&assign.target)),
        Stmt::ClassDef(class) => vec![class.name.to_string()],
        Stmt::For(loop_) => stored(from_ref(&loop_.target)),
        Stmt::FunctionDef(function) => vec![function.name.to_string()],
        Stmt::Import(import) => import
            .names
            .iter()
            .map(|alias| segment(alias.asname.as_ref().unwrap_or(&alias.name)))
            .collect(),
        Stmt::ImportFrom(import) => import
            .names
            .iter()
            .filter(|alias| alias.name.as_str() != "*")
            .map(|alias| segment(alias.asname.as_ref().unwrap_or(&alias.name)))
            .collect(),
        Stmt::TypeAlias(alias) => stored(from_ref(&alias.name)),
        Stmt::With(with) => with
            .items
            .iter()
            .filter_map(|item| item.optional_vars.as_deref())
            .flat_map(|target| stored(from_ref(target)))
            .collect(),
        other => walrus_targets(other),
    }
}

/// The first dotted segment of an imported name.
fn segment(name: &str) -> String {
    name.split_once('.')
        .map_or(name, |(first, _)| first)
        .to_owned()
}

/// The expressions one statement holds directly, which is where a walrus
/// can sit outside a nested body.
fn statement_expressions(statement: &Stmt) -> Vec<&Expr> {
    match statement {
        Stmt::Assert(assert) => vec![&assert.test],
        Stmt::Expr(expr) => vec![&expr.value],
        Stmt::If(branch) => vec![&branch.test],
        Stmt::Match(matched) => vec![matched.subject.as_ref()],
        Stmt::Return(returned) => returned.value.as_deref().into_iter().collect(),
        Stmt::While(loop_) => vec![&loop_.test],
        _ => Vec::new(),
    }
}

/// The names a run of targets stores, reaching into a tuple or list target
/// and leaving a subscript or attribute alone.
fn stored(targets: &[Expr]) -> Vec<String> {
    targets
        .iter()
        .flat_map(|target| match target {
            Expr::List(list) => stored(&list.elts),
            Expr::Name(name) if name.ctx == ExprContext::Store => vec![name.id.to_string()],
            Expr::Starred(starred) => stored(from_ref(&starred.value)),
            Expr::Tuple(tuple) => stored(&tuple.elts),
            _ => Vec::new(),
        })
        .collect()
}

/// The names a statement binds through a walrus anywhere in its own
/// expressions, which is what a statement kind binding nothing else can
/// still leave behind.
fn walrus_targets(statement: &Stmt) -> Vec<String> {
    let mut names = Vec::new();
    let mut walk = |expr: &Expr| {
        if let Expr::Named(named) = expr
            && let Expr::Name(name) = named.target.as_ref()
        {
            names.push(name.id.to_string());
        }
        false
    };
    for expr in statement_expressions(statement) {
        any_over_expr(expr, &mut walk);
    }
    names
}
