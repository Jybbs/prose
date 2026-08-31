//! Comparing what two trees left behind, meaning why one module's run counts
//! as broken beside the original, and which modules of a sweep broke at all.

use std::collections::{BTreeMap, BTreeSet};

use crate::records::{Break, Outcome};

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
    let reading = |held: &BTreeMap<String, Outcome>, module: &String| {
        held.get(module)
            .cloned()
            .unwrap_or_else(|| Outcome::of("unmeasured", "left no outcome at all".to_owned()))
    };
    let comparable: Vec<_> = modules
        .iter()
        .filter(|module| {
            reading(before, module).kind == "ok" && reading(after, module).kind != "unmeasured"
        })
        .cloned()
        .collect();
    let breaks = comparable
        .iter()
        .filter_map(|module| {
            let formatted = reading(after, module);
            let original = reading(before, module);
            let (reason, name) = divergence(&formatted, &original)?;
            Some(Break {
                attribution: String::new(),
                formatted,
                frame: (String::new(), None),
                hunk: Vec::new(),
                module: module.clone(),
                name,
                original,
                reason,
            })
        })
        .collect();
    let unmeasured = modules
        .iter()
        .filter(|module| {
            reading(before, module).kind == "unmeasured"
                || reading(after, module).kind == "unmeasured"
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
    if formatted.kind != "ok" {
        return Some((formatted.error.clone(), formatted.name.clone()));
    }
    let bound: BTreeSet<_> = original.names.iter().collect();
    let rebound: BTreeSet<_> = formatted.names.iter().collect();
    if let Some(name) = bound.difference(&rebound).min() {
        return Some((format!("leaves `{name}` unbound"), Some((*name).clone())));
    }
    if let Some(name) = rebound.difference(&bound).min() {
        return Some((
            format!("binds `{name}` the original does not"),
            Some((*name).clone()),
        ));
    }
    let differing = original
        .constants
        .keys()
        .chain(formatted.constants.keys())
        .filter(|name| original.constants.get(*name) != formatted.constants.get(*name))
        .min()?;
    let missing = MISSING.to_owned();
    let was = original.constants.get(differing).unwrap_or(&missing);
    let now = formatted.constants.get(differing).unwrap_or(&missing);
    Some((
        format!("binds `{differing}` to {now} where the original binds {was}"),
        Some(differing.clone()),
    ))
}
