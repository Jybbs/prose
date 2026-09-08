//! Tests for the comparison of two runs of one module, covering why the
//! formatted run counts as broken beside the original and how a width
//! sorts its candidates into buckets.

use itertools::Itertools;
use rstest::rstest;

use crate::{
    compare::{Divergence, compare, divergence},
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
        Some(Divergence {
            kind: "rebound",
            names: vec!["N".to_owned()],
            reason: why.to_owned(),
        })
    );
}

#[test]
fn a_dropped_name_and_an_added_name_report_their_direction() {
    let original = bound(&["a", "b"], &[]);
    let formatted = bound(&["a"], &[]);
    assert_eq!(
        divergence(&formatted, &original),
        Some(Divergence {
            kind: "unbound",
            names: vec!["b".to_owned()],
            reason: "leaves `b` unbound".to_owned(),
        })
    );
    assert_eq!(
        divergence(&original, &formatted),
        Some(Divergence {
            kind: "extra",
            names: vec!["b".to_owned()],
            reason: "binds `b` the original does not".to_owned(),
        })
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
        Some(Divergence {
            kind: "raises",
            names: vec!["x".to_owned()],
            reason: raised.error.clone(),
        })
    );
}

#[rstest]
#[case::one(&["a", "b"], "leaves `b` unbound")]
#[case::two(&["a", "b", "c"], "leaves `b` and 1 more name unbound")]
#[case::several(&["a", "b", "c", "d"], "leaves `b` and 2 more names unbound")]
fn a_run_losing_several_names_carries_every_one(#[case] original: &[&str], #[case] why: &str) {
    let diverged = divergence(&bound(&["a"], &[]), &bound(original, &[])).expect("diverges");
    assert_eq!(diverged.reason, why);
    assert_eq!(diverged.names, &original[1..]);
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
    assert_eq!(
        found.uncomparable.keys().collect::<Vec<_>>(),
        ["blocked.py"]
    );
    assert_eq!(
        found.uncomparable["blocked.py"].reason,
        "raises ImportError: no _abc"
    );
    assert_eq!(found.breaks.len(), 1);
    assert_eq!(found.breaks[0].module, "gone.py");
    assert_eq!(found.breaks[0].reason, "leaves `b` unbound");
}

#[test]
fn identical_namespaces_do_not_diverge() {
    let same = bound(&["N"], &[("N", "1")]);
    assert_eq!(divergence(&same, &same), None);
}
