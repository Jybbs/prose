//! Unit coverage for the harness's own arithmetic, one file per surface
//! and the helpers they share held here. That arithmetic decides which
//! rule a break is blamed on, so an off-by-one in it misattributes the
//! break instead of failing a test.

use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;

use crate::{
    outcome::{Kind, Outcome},
    records::{Blocked, Break, Frame},
};

mod bindings;
mod compare;
mod corpus;
mod execute;
mod fixes;
mod outcome;
mod ratchet;
mod records;
mod report;
mod sweep;

/// A module the original tree did not run cleanly, whose run raised
/// `raised` and reads as `reason`.
fn blocked(raised: &str, reason: &str) -> Blocked {
    Blocked {
        raised: raised.to_owned(),
        reason: reason.to_owned(),
    }
}

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

/// A break at `frame` for `module`, diverging for `reason`.
fn broken(module: &str, frame: &str, reason: &str) -> Break {
    Break {
        attribution: String::new(),
        formatted: Outcome::default(),
        frame: Frame {
            file: frame.to_owned(),
            row: None,
        },
        hunk: Vec::new(),
        kind: "unbound",
        module: module.to_owned(),
        name: None,
        names: Vec::new(),
        original: Outcome::default(),
        reason: reason.to_owned(),
    }
}

/// A break at `frame` for `module`, losing `name` and nothing else.
fn losing(module: &str, frame: &str, name: &str) -> Break {
    Break {
        names: vec![name.to_owned()],
        ..broken(module, frame, &format!("leaves `{name}` unbound"))
    }
}

/// The uncomparable map holding each of `modules`, every one raising a
/// plain `ImportError`, which is the shape a case reaches for wherever
/// only the module name carries the assertion.
fn stalled<const N: usize>(modules: [&str; N]) -> BTreeMap<String, Blocked> {
    modules
        .iter()
        .map(|module| ((*module).to_owned(), blocked("ImportError", "raises")))
        .collect()
}

/// The flaky map holding each of `modules` beside the names it varies on,
/// an empty slice standing for a module whose entire namespace varied.
fn varied<const N: usize>(modules: [(&str, &[&str]); N]) -> BTreeMap<String, BTreeSet<String>> {
    modules
        .iter()
        .map(|(module, names)| {
            (
                (*module).to_owned(),
                names.iter().map(|name| (*name).to_owned()).collect(),
            )
        })
        .collect()
}
