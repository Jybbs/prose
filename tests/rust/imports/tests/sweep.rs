//! Tests for the two verdicts one sweep reaches on a break, covering the
//! rerun of the original that never ended cleanly and, once the names a
//! module varies on are set aside, the comparison the formatted rerun
//! still has to pass.

use std::{assert_matches, collections::BTreeSet};

use rstest::rstest;

use super::*;
use crate::{
    outcome::{Kind, Outcome},
    sweep::{Confirmed, settled, verdict},
};

#[test]
fn a_difference_confined_to_the_varying_names_leaves_the_break_unproven() {
    let varies = ["_cast_addr".to_owned()].into();
    let before = bound(&["_cast_addr", "cdll"], &[("_cast_addr", "0x10a")]);
    let after = bound(&["_cast_addr", "cdll"], &[("_cast_addr", "0x2f8")]);
    assert_matches!(verdict(&after, &before, varies), Confirmed::Flaky(names) if names.len() == 1);
}

#[test]
fn a_difference_outside_the_varying_names_still_breaks() {
    let varies = ["_cast_addr".to_owned()].into();
    let before = bound(&["_cast_addr", "cdll"], &[("_cast_addr", "0x10a")]);
    let after = bound(&["_cast_addr"], &[("_cast_addr", "0x2f8")]);
    assert_matches!(verdict(&after, &before, varies), Confirmed::Break(names) if names.len() == 1);
}

#[test]
fn a_formatted_rerun_leaving_no_record_measures_nothing() {
    let before = bound(&["cdll"], &[]);
    let after = Outcome::of(Kind::Unmeasured, "leaves no record");
    assert_matches!(
        verdict(&after, &before, BTreeSet::new()),
        Confirmed::Unmeasured
    );
}

#[rstest]
#[case::raised(Kind::Raised)]
#[case::timed_out(Kind::Timeout)]
fn a_formatted_rerun_that_never_bound_a_namespace_breaks_past_any_variance(#[case] kind: Kind) {
    let varies = ["_cast_addr".to_owned()].into();
    let before = bound(&["_cast_addr", "cdll"], &[("_cast_addr", "0x10a")]);
    let after = Outcome::of(kind, "never bound a namespace");
    assert_matches!(verdict(&after, &before, varies), Confirmed::Break(names) if names.len() == 1);
}

#[rstest]
#[case::timed_out(Kind::Timeout)]
#[case::left_no_record(Kind::Unmeasured)]
fn a_rerun_that_never_ended_cleanly_measures_nothing(#[case] kind: Kind) {
    assert_matches!(settled(kind), Some(Confirmed::Unmeasured));
}

#[test]
fn a_rerun_that_raised_varies_across_its_whole_namespace() {
    assert_matches!(settled(Kind::Raised), Some(Confirmed::Flaky(names)) if names.is_empty());
}

#[test]
fn a_rerun_that_ran_cleanly_leaves_the_formatted_side_worth_running() {
    assert_matches!(settled(Kind::Ok), None);
}
