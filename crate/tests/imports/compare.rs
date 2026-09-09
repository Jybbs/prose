//! Comparing what two trees left behind, meaning why one module's run counts
//! as broken beside the original, and which modules of a sweep broke at all.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    outcome::{Kind, Outcome},
    records::{Blocked, Break, Frame},
};

/// The reading a constant takes where the run bound no plain constant of
/// that name.
const MISSING: &str = "no plain constant";

/// Why one run counts as broken beside another, as the tag and names a
/// baseline keys on beside the sentence a report shows.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Divergence {
    /// What kind of difference this is, stable under any rewording.
    pub(crate) kind: &'static str,
    /// Every name the difference turns on, sorted.
    pub(crate) names: Vec<String>,
    /// The sentence a report shows.
    pub(crate) reason: String,
}

/// How one width's candidates divide, the breaks found among the comparable
/// ones beside the names the verdict reads.
pub(crate) struct Partition {
    /// Every module the rewrite breaks.
    pub(crate) breaks: Vec<Break>,
    /// How many modules the original tree ran cleanly.
    pub(crate) comparable: usize,
    /// The modules the original tree did not run cleanly, each beside
    /// what its run left, which a run therefore never judges.
    pub(crate) uncomparable: BTreeMap<String, Blocked>,
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
    let mut uncomparable: BTreeMap<String, Blocked> = BTreeMap::new();
    let mut unmeasured: Vec<String> = Vec::new();
    for module in modules {
        match (kind(before, module), kind(after, module)) {
            (Kind::Unmeasured, _) | (_, Kind::Unmeasured) => unmeasured.push(module.clone()),
            (Kind::Ok, _) => comparable.push(module.clone()),
            _ => {
                let left = before.get(module).map_or_else(
                    || Blocked {
                        raised: String::new(),
                        reason: "the original tree was never asked".to_owned(),
                    },
                    |ran| Blocked {
                        raised: ran.raised.clone(),
                        reason: ran.error.clone(),
                    },
                );
                uncomparable.insert(module.clone(), left);
            }
        }
    }
    let breaks = comparable
        .iter()
        .filter_map(|module| {
            let formatted = after.get(module)?;
            let original = before.get(module)?;
            let diverged = divergence(formatted, original)?;
            Some(Break {
                attribution: String::new(),
                formatted: formatted.clone(),
                frame: Frame::default(),
                hunk: Vec::new(),
                kind: diverged.kind,
                module: module.clone(),
                name: diverged.names.first().cloned(),
                names: diverged.names,
                original: original.clone(),
                reason: diverged.reason,
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

/// Says why one run counts as broken beside another, or `None` where both
/// bound the same namespace.
pub(crate) fn divergence(formatted: &Outcome, original: &Outcome) -> Option<Divergence> {
    if formatted.kind != Kind::Ok {
        return Some(Divergence {
            kind: if formatted.kind == Kind::Timeout {
                "times out"
            } else {
                "raises"
            },
            names: formatted.name.clone().into_iter().collect(),
            reason: formatted.error.clone(),
        });
    }
    let lost: Vec<_> = missing(&original.names, &formatted.names)
        .cloned()
        .collect();
    if let [name, rest @ ..] = lost.as_slice() {
        let reason = format!("leaves {} unbound", named(name, rest.len()));
        return Some(Divergence {
            kind: "unbound",
            names: lost,
            reason,
        });
    }
    let gained: Vec<_> = missing(&formatted.names, &original.names)
        .cloned()
        .collect();
    if let [name, rest @ ..] = gained.as_slice() {
        let reason = format!("binds {} the original does not", named(name, rest.len()));
        return Some(Divergence {
            kind: "extra",
            names: gained,
            reason,
        });
    }
    let differing = respelt(original, formatted).min()?;
    let was = original
        .constants
        .get(differing)
        .map_or(MISSING, String::as_str);
    let now = formatted
        .constants
        .get(differing)
        .map_or(MISSING, String::as_str);
    Some(Divergence {
        kind: "rebound",
        names: vec![differing.clone()],
        reason: format!("binds `{differing}` to {now} where the original binds {was}"),
    })
}

/// Every name two runs of one tree bound differently, covering a name only
/// one side bound and a constant the two spelt differently. [`divergence`]
/// stops at the first of these it finds, where this returns every one.
pub(crate) fn varying(one: &Outcome, other: &Outcome) -> BTreeSet<String> {
    missing(&one.names, &other.names)
        .chain(missing(&other.names, &one.names))
        .chain(respelt(one, other))
        .cloned()
        .collect()
}

/// The kind a run of one module left behind, `unmeasured` where the tree was
/// never asked about it.
fn kind(held: &BTreeMap<String, Outcome>, module: &str) -> Kind {
    held.get(module)
        .map_or(Kind::Unmeasured, |outcome| outcome.kind)
}

/// The names of `from` that `held` does not carry. Both slices arrive
/// sorted, which is what lets the lookup bisect `held`.
fn missing<'a>(from: &'a [String], held: &'a [String]) -> impl Iterator<Item = &'a String> {
    from.iter()
        .filter(move |name| held.binary_search(name).is_err())
}

/// One name and however many followed it, which is the sentence a report
/// shows rather than the key a baseline holds.
fn named(first: &str, rest: usize) -> String {
    match rest {
        0 => format!("`{first}`"),
        1 => format!("`{first}` and 1 more name"),
        _ => format!("`{first}` and {rest} more names"),
    }
}

/// The names under which two runs spelt a plain constant differently. A
/// name both runs carry arrives twice.
fn respelt<'a>(one: &'a Outcome, other: &'a Outcome) -> impl Iterator<Item = &'a String> {
    one.constants
        .keys()
        .chain(other.constants.keys())
        .filter(move |name| one.constants.get(*name) != other.constants.get(*name))
}
