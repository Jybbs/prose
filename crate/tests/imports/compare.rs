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

/// How one width's candidates divide, the breaks found among the comparable
/// ones beside the names the verdict reads.
pub(crate) struct Partition {
    /// Every module the rewrite breaks.
    pub(crate) breaks: Vec<Break>,
    /// How many modules the original tree ran cleanly.
    pub(crate) comparable: usize,
    /// The modules the original tree did not run cleanly, which a run
    /// therefore never judges.
    pub(crate) uncomparable: Vec<String>,
    /// The modules a run left no record for.
    pub(crate) unmeasured: Vec<String>,
}

/// Finds the modules the rewrite breaks, and how the rest divide. Each
/// module lands in exactly one bucket, the classification reading both
/// runs once rather than restating itself per bucket.
pub(crate) fn compare(
    after: &BTreeMap<String, Outcome>,
    before: &BTreeMap<String, Outcome>,
    modules: &[String],
) -> Partition {
    let mut comparable: Vec<String> = Vec::new();
    let mut uncomparable: Vec<String> = Vec::new();
    let mut unmeasured: Vec<String> = Vec::new();
    for module in modules {
        let bucket = match (kind(before, module), kind(after, module)) {
            (Kind::Unmeasured, _) | (_, Kind::Unmeasured) => &mut unmeasured,
            (Kind::Ok, _) => &mut comparable,
            _ => &mut uncomparable,
        };
        bucket.push(module.clone());
    }
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
    Partition {
        breaks,
        comparable: comparable.len(),
        uncomparable,
        unmeasured,
    }
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
    let missing = |from: &[String], held: &[String]| -> Vec<String> {
        from.iter()
            .filter(|name| held.binary_search(name).is_err())
            .cloned()
            .collect()
    };
    if let [name, rest @ ..] = missing(&original.names, &formatted.names).as_slice() {
        let reason = format!("leaves {} unbound", named(name, rest.len()));
        return Some((reason, Some(name.clone())));
    }
    if let [name, rest @ ..] = missing(&formatted.names, &original.names).as_slice() {
        let reason = format!("binds {} the original does not", named(name, rest.len()));
        return Some((reason, Some(name.clone())));
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

/// One name and however many followed it, so a run losing several names
/// keys on the count rather than on the first name alone.
fn named(first: &str, rest: usize) -> String {
    match rest {
        0 => format!("`{first}`"),
        1 => format!("`{first}` and 1 more name"),
        _ => format!("`{first}` and {rest} more names"),
    }
}

/// The kind a run of one module left behind, `unmeasured` where the tree was
/// never asked about it.
fn kind(held: &BTreeMap<String, Outcome>, module: &str) -> Kind {
    held.get(module)
        .map_or(Kind::Unmeasured, |outcome| outcome.kind)
}
