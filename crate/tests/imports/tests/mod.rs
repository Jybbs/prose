//! Unit coverage for the harness's own arithmetic, one file per surface
//! and the helpers they share held here. That arithmetic decides which
//! rule a break is blamed on, so an off-by-one in it misattributes the
//! break instead of failing a test.

use crate::{
    outcome::Outcome,
    records::{Break, Frame},
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
