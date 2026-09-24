//! Tests that no rule run alone moves a statement of a class body under
//! a `# prose: keep` header, over each corpus input.

use std::collections::BTreeSet;

use ruff_python_ast::Stmt;
use rustc_hash::FxHashMap;

use super::*;
use crate::{
    primitives::{
        binding::module_bound_names,
        comments::class_keeps_order,
        scope::{scoped_body, sub_bodies},
    },
    testing::corpus_inputs,
};

/// The statements of one kept class body in the order written, the arms
/// of each compound statement read in place, each as the names it binds.
type Entries<'a> = Vec<Vec<&'a str>>;

#[test]
fn no_rule_alone_moves_a_statement_of_a_kept_class_body() {
    let config = Config::default();
    let mut checked = false;
    for path in corpus_inputs() {
        let Ok(source) = Source::from_path(&path) else {
            continue;
        };
        let written = kept_bodies(&source);
        if written.is_empty() {
            continue;
        }
        for &rule in Pipeline::known_ids() {
            let pipeline = Pipeline::with_filters(&config, &[rule], &[]);
            let Ok(formatted) = pipeline.format(source.clone()) else {
                continue;
            };
            for (class, after) in kept_bodies(&formatted) {
                let Some(before) = written.get(&class) else {
                    continue;
                };
                let site = format!("`{rule}` over `{class}` in {}", path.display());
                assert_holds_order(before, &after, &site);
                checked = true;
            }
        }
    }
    assert!(checked, "no kept class body was checked across the corpus");
}

/// Asserts that each statement of `after` binds the names of a run of
/// consecutive statements of `before`, each run opening at or past the
/// close of the one ahead of it. A deletion, a merge of neighbors, and a
/// split all pass, whereas a move fails, a name `before` binds twice
/// counting toward no statement.
fn assert_holds_order(before: &Entries<'_>, after: &Entries<'_>, site: &str) {
    let mut owner: FxHashMap<&str, Option<usize>> = FxHashMap::default();
    for (slot, names) in before.iter().enumerate() {
        for &name in names {
            owner
                .entry(name)
                .and_modify(|seen| *seen = None)
                .or_insert(Some(slot));
        }
    }
    let mut floor = 0;
    for names in after {
        let slots: BTreeSet<usize> = names
            .iter()
            .filter_map(|name| owner.get(name).copied().flatten())
            .collect();
        let (Some(&first), Some(&last)) = (slots.first(), slots.last()) else {
            continue;
        };
        assert!(
            first >= floor && last - first + 1 == slots.len(),
            "{site} moves the statement binding {names:?}",
        );
        floor = last;
    }
}

/// Inserts each class under a `# prose: keep` header in `body` into
/// `kept`, keyed by its dotted path through the definitions enclosing it,
/// beside the entries of its body.
fn collect_kept<'a>(
    source: &Source,
    body: &'a [Stmt],
    prefix: &str,
    kept: &mut FxHashMap<String, Entries<'a>>,
) {
    for stmt in body {
        let name = match stmt {
            Stmt::ClassDef(class) => Some(class.name.as_str()),
            Stmt::FunctionDef(function) => Some(function.name.as_str()),
            _ => None,
        };
        let path = match name {
            Some(name) if prefix.is_empty() => name.to_owned(),
            Some(name) => format!("{prefix}.{name}"),
            None => prefix.to_owned(),
        };
        if let Stmt::ClassDef(class) = stmt
            && class_keeps_order(source, class)
        {
            let mut entries = Vec::new();
            flatten(&class.body, &mut entries);
            kept.insert(path.clone(), entries);
        }
        for (sub, _) in sub_bodies(stmt) {
            collect_kept(source, sub, &path, kept);
        }
    }
}

/// Appends the names each statement of `body` binds to `entries`, a
/// compound statement contributing the statements of its arms in place
/// of an entry of its own.
fn flatten<'a>(body: &'a [Stmt], entries: &mut Entries<'a>) {
    for stmt in body {
        let arms = sub_bodies(stmt);
        if scoped_body(stmt).is_some() || arms.is_empty() {
            entries.push(module_bound_names(stmt));
        } else {
            for (arm, _) in arms {
                flatten(arm, entries);
            }
        }
    }
}

/// Returns every kept class body in `source`, per [`collect_kept`].
fn kept_bodies(source: &Source) -> FxHashMap<String, Entries<'_>> {
    let mut kept = FxHashMap::default();
    collect_kept(source, &source.ast().body, "", &mut kept);
    kept
}
