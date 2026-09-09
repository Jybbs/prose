//! Tests for the walk that records where a module binds each name,
//! covering which statement kinds bind one and which scopes the walk
//! leaves alone.

use std::collections::BTreeMap;

use rstest::rstest;

use crate::bindings::binding_rows;

/// A module binding at every compound-statement arm, with a function,
/// a class, and a parameter binding names it does not.
const NESTED_SCOPES: &str = "import os.path as osp\nfrom re import compile as rc\n\n\ndef f(a):\n    inner = 1\n\n\nclass K:\n    attr = 2\n\n\ntry:\n    t = 1\nexcept ValueError:\n    e = 2\n\nfor i in y:\n    pass\n";

#[test]
fn a_decorated_definition_binds_on_its_def_row_rather_than_its_decorator() {
    let rows = binding_rows("@deco\n@other\ndef f():\n    pass\n\n\n@deco\nclass K:\n    pass\n");
    assert_eq!(rows.get("f"), Some(&(3..4)));
    assert_eq!(rows.get("K"), Some(&(8..9)));
}

#[test]
fn a_walrus_and_a_type_alias_bind_at_module_level() {
    let rows = binding_rows(
        "if (n := go()):\n    pass\n\nwhile (m := go()):\n    pass\n\ntype Alias = int\n",
    );
    assert_eq!(rows.get("n"), Some(&(1..3)));
    assert_eq!(rows.get("m"), Some(&(4..6)));
    assert_eq!(rows.get("Alias"), Some(&(7..8)));
}

#[test]
fn a_walrus_in_a_match_subject_binds_at_module_level() {
    let rows = binding_rows("match (n := go()):\n    case _:\n        pass\n");
    assert_eq!(rows.get("n"), Some(&(1..4)));
}

#[test]
fn an_import_binds_its_first_dotted_segment() {
    let rows =
        binding_rows("import os.path\nimport xml.etree.ElementTree as et\nfrom a.b import c\n");
    assert_eq!(rows.get("os"), Some(&(1..2)));
    assert_eq!(rows.get("et"), Some(&(2..3)));
    assert_eq!(rows.get("c"), Some(&(3..4)));
    assert!(!rows.contains_key("xml"));
}

#[test]
fn binding_rows_are_empty_for_a_module_that_does_not_parse() {
    assert_eq!(binding_rows("def (\n"), BTreeMap::new());
}

#[rstest]
fn binding_rows_reach_tuple_and_starred_targets(
    #[values("STRICT", "CONFORM", "head", "rest", "a", "b")] name: &str,
) {
    let rows = binding_rows("STRICT, CONFORM = boundary()\nhead, *rest = xs\n[a, b] = pair\n");
    assert!(rows.contains_key(name), "{name} binds at module level");
}

#[rstest]
fn binding_rows_skip_a_nested_scope(#[values("inner", "attr", "a")] absent: &str) {
    assert!(
        !binding_rows(NESTED_SCOPES).contains_key(absent),
        "{absent} binds in a nested scope"
    );
}

#[test]
fn binding_rows_walk_compound_statements() {
    let rows = binding_rows(NESTED_SCOPES);
    assert_eq!(rows.get("osp"), Some(&(1..2)));
    assert_eq!(rows.get("rc"), Some(&(2..3)));
    assert_eq!(rows.get("f"), Some(&(5..6)));
    assert_eq!(rows.get("K"), Some(&(9..10)));
    assert_eq!(rows.get("t"), Some(&(14..15)));
    assert_eq!(rows.get("e"), Some(&(16..17)));
    assert_eq!(rows.get("i"), Some(&(18..20)));
}
