//! Comparing what two trees left behind, meaning why one module's run counts
//! as broken beside the original, and which modules of a sweep broke at all.

use std::collections::BTreeMap;

use crate::{
    outcome::{Kind, Outcome},
    records::{Break, Frame},
};

/// The reading a constant takes where the run bound no plain constant of
/// that name.
const MISSING: &str = "no plain constant";

/// Finds the modules the rewrite breaks.
///
/// Returns the breaks, the count of modules comparable at all, and the ones a
/// run left no record for.
pub(crate) fn compare(
    after: &BTreeMap<String, Outcome>,
    before: &BTreeMap<String, Outcome>,
    modules: &[String],
) -> (Vec<Break>, usize, Vec<String>) {
    let comparable: Vec<_> = modules
        .iter()
        .filter(|module| {
            kind(before, module) == Kind::Ok && kind(after, module) != Kind::Unmeasured
        })
        .cloned()
        .collect();
    let breaks = comparable
        .iter()
        .filter_map(|module| {
            let formatted = after.get(module)?;
            let original = before.get(module)?;
            let (reason, name) = divergence(formatted, original)?;
            Some(Break {
                attribution: String::new(),
                formatted: formatted.clone(),
                frame: Frame::default(),
                hunk: Vec::new(),
                module: module.clone(),
                name,
                original: original.clone(),
                reason,
            })
        })
        .collect();
    let unmeasured = modules
        .iter()
        .filter(|module| {
            kind(before, module) == Kind::Unmeasured || kind(after, module) == Kind::Unmeasured
        })
        .cloned()
        .collect();
    (breaks, comparable.len(), unmeasured)
}

/// Says why one run counts as broken beside another and the name it turns on,
/// or `None` where both bound the same namespace.
pub(crate) fn divergence(
    formatted: &Outcome,
    original: &Outcome,
) -> Option<(String, Option<String>)> {
    if formatted.kind != Kind::Ok {
        return Some((formatted.error.clone(), formatted.name.clone()));
    }
    let missing = |from: &[String], held: &[String]| {
        from.iter()
            .find(|name| held.binary_search(name).is_err())
            .cloned()
    };
    if let Some(name) = missing(&original.names, &formatted.names) {
        return Some((format!("leaves `{name}` unbound"), Some(name)));
    }
    if let Some(name) = missing(&formatted.names, &original.names) {
        return Some((format!("binds `{name}` the original does not"), Some(name)));
    }
    let differing = original
        .constants
        .keys()
        .chain(formatted.constants.keys())
        .filter(|name| original.constants.get(*name) != formatted.constants.get(*name))
        .min()?;
    let was = original
        .constants
        .get(differing)
        .map_or(MISSING, String::as_str);
    let now = formatted
        .constants
        .get(differing)
        .map_or(MISSING, String::as_str);
    Some((
        format!("binds `{differing}` to {now} where the original binds {was}"),
        Some(differing.clone()),
    ))
}

/// The kind a run of one module left behind, `unmeasured` where the tree was
/// never asked about it.
fn kind(held: &BTreeMap<String, Outcome>, module: &str) -> Kind {
    held.get(module)
        .map_or(Kind::Unmeasured, |outcome| outcome.kind)
}
