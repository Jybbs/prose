//! Tests for the comparison of two runs of one module, covering why the
//! formatted run counts as broken beside the original and how a width
//! sorts its candidates into buckets.

use std::collections::BTreeSet;

use rstest::rstest;

use super::*;
use crate::{
    compare::{Divergence, compare, divergence, varying},
    outcome::{Kind, Outcome, Raise},
};

#[test]
fn a_constant_only_one_run_binds_counts_as_varying() {
    assert_eq!(
        varying(&bound(&["N"], &[("N", "1")]), &bound(&["N"], &[])),
        ["N".to_owned()].into()
    );
}

#[rstest]
#[case::rebound(&[("N", "2")], "binds `N` to 2 where the original binds 1")]
#[case::no_longer_plain(&[], "binds `N` to no plain constant where the original binds 1")]
fn a_constant_rebound_names_both_values(#[case] spelt: &[(&str, &str)], #[case] why: &str) {
    assert_eq!(
        divergence(&bound(&["N"], spelt), &bound(&["N"], &[("N", "1")])),
        Some(Divergence {
            names: vec!["N".to_owned()],
            reason: why.to_owned(),
        })
    );
}

#[test]
fn a_definition_raising_in_one_run_alone_or_differently_counts_as_varying() {
    assert_eq!(
        varying(
            &unevaluating(&[
                ("C.m", "NameError", "A"),
                ("f", "NameError", "B"),
                ("g", "NameError", "C")
            ]),
            &unevaluating(&[("f", "NameError", "B"), ("g", "NameError", "D")])
        ),
        ["C.m".to_owned(), "g".to_owned()].into()
    );
}

#[test]
fn a_definition_raising_the_same_way_in_the_original_is_left_out() {
    let raising = unevaluating(&[("f", "NameError", "A")]);
    assert_eq!(divergence(&raising, &raising), None);
    assert_eq!(divergence(&unevaluating(&[]), &raising), None);
}

#[test]
fn a_definition_raising_on_another_name_than_the_original_breaks_on_that_name() {
    let diverged = divergence(
        &unevaluating(&[("f", "NameError", "B")]),
        &unevaluating(&[("f", "NameError", "A")]),
    )
    .expect("diverges");
    assert_eq!(diverged.names, ["B"]);
    assert_eq!(
        diverged.reason,
        "reading the annotations of `f` raises NameError on `B` where the original's do not"
    );
}

#[rstest]
#[case::one(
    &[("C.m", "NameError", "Sequence")],
    &["Sequence"],
    "reading the annotations of `C.m` raises NameError on `Sequence` where the original's do not"
)]
#[case::two(
    &[("C", "NameError", "A"), ("f", "AttributeError", "b")],
    &["A", "b"],
    "reading the annotations of `C` and 1 more definition raises NameError on `A` where the \
     original's do not"
)]
#[case::unnamed(
    &[("f", "ValueError", "")],
    &["f"],
    "reading the annotations of `f` raises ValueError where the original's do not"
)]
fn a_definition_raising_only_when_formatted_breaks_on_the_name_it_misses(
    #[case] raising: &[(&str, &str, &str)],
    #[case] names: &[&str],
    #[case] why: &str,
) {
    assert_eq!(
        divergence(&unevaluating(raising), &unevaluating(&[])),
        Some(Divergence {
            names: names.iter().map(|name| (*name).to_owned()).collect(),
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
            names: vec!["b".to_owned()],
            reason: "leaves `b` unbound".to_owned(),
        })
    );
    assert_eq!(
        divergence(&original, &formatted),
        Some(Divergence {
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
fn an_annotations_divergence_reads_as_what_the_module_annotates() {
    let was = bound(&[], &[("__annotations__", "('a', 'b')")]);
    let now = bound(&[], &[("__annotations__", "('a',)")]);
    assert_eq!(
        divergence(&now, &was).map(|diverged| diverged.reason),
        Some("annotates ('a',) at module scope where the original annotates ('a', 'b')".to_owned())
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

#[test]
fn varying_reads_both_directions_and_the_constants_where_divergence_stops_at_one() {
    let one = bound(&["N", "a", "b"], &[("N", "1")]);
    let other = bound(&["N", "a", "c"], &[("N", "2")]);
    assert_eq!(
        varying(&one, &other),
        ["N".to_owned(), "b".to_owned(), "c".to_owned()].into()
    );
    assert_eq!(
        divergence(&one, &other).expect("diverges").names,
        ["c".to_owned()]
    );
    assert_eq!(varying(&one, &one), BTreeSet::new());
}

/// A run binding `C`, `f`, and `g` whose definitions in `raising` raised the
/// exception beside each on the name after it, an empty name standing for
/// none, when their annotations were read.
fn unevaluating(raising: &[(&str, &str, &str)]) -> Outcome {
    Outcome {
        unevaluated: raising
            .iter()
            .map(|(held, raised, missing)| {
                let raise = Raise {
                    missing: Some((*missing).to_owned()).filter(|name| !name.is_empty()),
                    raised: (*raised).to_owned(),
                };
                ((*held).to_owned(), raise)
            })
            .collect(),
        ..bound(&["C", "f", "g"], &[])
    }
}
