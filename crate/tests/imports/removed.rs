//! The names a comparison leaves out, meaning each name a module's original
//! bound and its formatted copy does not, where a recorded fix removed the
//! binding.

use std::{collections::BTreeMap, path::Path};

use crate::{
    bindings::bound_at,
    compare::missing,
    fixes::{fitting, unbinds},
    outcome::{Kind, Outcome},
    records::{Fixes, Removed},
};

/// The clause naming where `name` was bound in `module` of `original` and the
/// rules whose recorded fixes removed that binding, `None` where no fix did.
pub(crate) fn dropped(fixes: &Fixes, original: &Path, module: &str, name: &str) -> Option<String> {
    let (rows, text) = bound_at(original, module, name)?;
    let rules = fitting(fixes, module, |edits| unbinds(edits, &rows, name, &text))?;
    Some(format!(
        "`{name}` bound at {module}:{}, dropped by {rules}",
        rows.start
    ))
}

/// The names each module's original bound and its formatted copy does not,
/// where both ran cleanly and a recorded fix removed the binding in the
/// original, each beside the clause [`dropped`] writes for it.
pub(crate) fn removed(
    after: &BTreeMap<String, Outcome>,
    before: &BTreeMap<String, Outcome>,
    fixes: &Fixes,
    original: &Path,
) -> Removed {
    fixes
        .keys()
        .filter_map(|module| {
            let formatted = after.get(module).filter(|ran| ran.kind == Kind::Ok)?;
            let ran = before.get(module).filter(|ran| ran.kind == Kind::Ok)?;
            let left: BTreeMap<_, _> = missing(&ran.names, &formatted.names)
                .filter_map(|name| Some((name.clone(), dropped(fixes, original, module, name)?)))
                .collect();
            (!left.is_empty()).then(|| (module.clone(), left))
        })
        .collect()
}
