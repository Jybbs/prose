//! Tests for the comparison of two runs of one module, covering why the
//! formatted run counts as broken beside the original and how a width
//! sorts its candidates into buckets.

use itertools::Itertools;
use rstest::rstest;

use crate::{
    compare::{compare, divergence},
    outcome::{Kind, Outcome},
};

/// An outcome that ran cleanly, binding `names` and the constants `spelt`.
fn bound(names: &[&str], spelt: &[(&str, &str)]) -> Outcome {
    Outcome {
        constants: spelt
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        kind: Kind::Ok,
        names: names
            .iter()
            .map(|name| (*name).to_owned())
            .sorted()
            .collect(),
        ..Outcome::default()
    }
}

#[rstest]
#[case::rebound(&[("N", "2")], "binds `N` to 2 where the original binds 1")]
#[case::no_longer_plain(&[], "binds `N` to no plain constant where the original binds 1")]
fn a_constant_rebound_names_both_values(#[case] spelt: &[(&str, &str)], #[case] why: &str) {
    assert_eq!(
        divergence(&bound(&["N"], spelt), &bound(&["N"], &[("N", "1")])),
        Some((why.to_owned(), Some("N".to_owned())))
    );
}

#[test]
fn a_dropped_name_and_an_added_name_report_their_direction() {
    let original = bound(&["a", "b"], &[]);
    let formatted = bound(&["a"], &[]);
    assert_eq!(
        divergence(&formatted, &original),
        Some(("leaves `b` unbound".to_owned(), Some("b".to_owned())))
    );
    assert_eq!(
        divergence(&original, &formatted),
        Some((
            "binds `b` the original does not".to_owned(),
            Some("b".to_owned())
        ))
    );
}

#[test]
fn a_raised_run_returns_its_error_and_name() {
    let raised = Outcome {
        error: "raises NameError: name 'x' is not defined".to_owned(),
        kind: Kind::Raised,
        name: Some("x".to_owned()),
        ..Outcome::default()
    };
    assert_eq!(
        divergence(&raised, &bound(&[], &[])),
        Some((raised.error.clone(), Some("x".to_owned())))
    );
}

#[test]
fn comparing_sorts_each_module_into_one_bucket() {
    let raised = || Outcome::of(Kind::Raised, "raises ImportError: no _abc");
    let modules = [
        "blocked.py".to_owned(),
        "gone.py".to_owned(),
        "kept.py".to_owned(),
        "lost.py".to_owned(),
    ];
    let before = [
        ("blocked.py".to_owned(), raised()),
        ("gone.py".to_owned(), bound(&["a", "b"], &[])),
        ("kept.py".to_owned(), bound(&["a"], &[])),
        ("lost.py".to_owned(), bound(&["a"], &[])),
    ]
    .into();
    let after = [
        ("blocked.py".to_owned(), raised()),
        ("gone.py".to_owned(), bound(&["a"], &[])),
        ("kept.py".to_owned(), bound(&["a"], &[])),
        (
            "lost.py".to_owned(),
            Outcome::of(Kind::Unmeasured, "left no record"),
        ),
    ]
    .into();
    let found = compare(&after, &before, &modules);
    assert_eq!(found.comparable, 2);
    assert_eq!(found.unmeasured, ["lost.py".to_owned()]);
    assert_eq!(found.uncomparable, ["blocked.py".to_owned()]);
    assert_eq!(found.breaks.len(), 1);
    assert_eq!(found.breaks[0].module, "gone.py");
    assert_eq!(found.breaks[0].reason, "leaves `b` unbound");
}

#[test]
fn identical_namespaces_do_not_diverge() {
    let same = bound(&["N"], &[("N", "1")]);
    assert_eq!(divergence(&same, &same), None);
}
