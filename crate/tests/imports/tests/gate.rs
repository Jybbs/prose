//! Tests for the verdict a run reaches at one width, covering each cause
//! that fails it and the run that passes.

use std::{assert_matches, collections::BTreeMap};

use rstest::rstest;

use super::*;
use crate::{
    gate::failures,
    reach::Reach,
    records::{Blocked, Width},
};

/// A width that ran, rewrote, and compared one module and found nothing,
/// which a case overrides with the field it breaks.
fn clean() -> Width {
    Width {
        candidates: 1,
        comparable: 1,
        rewritten: 1,
        ..width()
    }
}

/// An uncomparable map holding `module` at `reach`.
fn reached(module: &str, reach: Reach) -> BTreeMap<String, Blocked> {
    [(
        module.to_owned(),
        Blocked {
            reach,
            reason: "raises ImportError: no thing".to_owned(),
        },
    )]
    .into()
}

#[test]
fn a_clean_width_fails_on_nothing() {
    assert_eq!(failures(&clean()), Vec::<String>::new());
}

#[rstest]
fn an_uncomparable_module_the_loader_did_not_fail_fails_nothing(
    #[values(Reach::Absent, Reach::Module, Reach::Platform)] reach: Reach,
) {
    let found = Width {
        uncomparable: reached("m.py", reach),
        ..clean()
    };
    assert_eq!(failures(&found), Vec::<String>::new());
}

#[rstest]
#[case::unmeasured(
    Width { unmeasured: vec!["u.py".to_owned()], ..clean() },
    "u.py left no record",
)]
#[case::rewrote_nothing(Width { rewritten: 0, ..clean() }, "the format pass rewrote no file")]
#[case::compared_nothing(Width { comparable: 0, ..clean() }, "the run compared no module")]
#[case::loader(
    Width { uncomparable: reached("pkg/mod.py", Reach::Loader), ..clean() },
    "the harness's loader failed pkg/mod.py",
)]
#[case::broken(
    Width { breaks: vec![losing("m.py", "m.py", "X")], ..clean() },
    "the rewrite breaks m.py",
)]
#[case::rejected(
    Width { rejected: [("s.pyi".to_owned(), "rule `x` failed".to_owned())].into(), ..clean() },
    "the pipeline could not format s.pyi",
)]
fn each_cause_fails_the_width_it_names(#[case] found: Width, #[case] cause: &str) {
    assert_matches!(failures(&found).as_slice(), [only] if only.contains(cause));
}

#[test]
fn several_modules_under_one_cause_name_the_first_and_count_the_rest() {
    let found = Width {
        breaks: vec![losing("a.py", "a.py", "X"), losing("b.py", "b.py", "Y")],
        rejected: [("c.pyi".to_owned(), "rule `x` failed".to_owned())].into(),
        ..clean()
    };
    assert_eq!(
        failures(&found),
        [
            "the pipeline could not format c.pyi",
            "the rewrite breaks a.py and 1 more module"
        ]
    );
}
