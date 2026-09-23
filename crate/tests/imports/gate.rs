//! The verdict a run reaches at one width, meaning every cause that fails
//! it, each named.

use crate::{compare::with_rest, reach::Reach, records::Width};

/// Every cause that fails one width's run, empty where it passes. A run
/// fails where a module left no record, where the format pass rewrote no
/// file, where it compared no module, where the loader rather than a module
/// failed that module, and on any break.
pub(crate) fn failures(found: &Width) -> Vec<String> {
    let loader = found
        .uncomparable
        .iter()
        .filter(|(_, left)| left.reach == Reach::Loader)
        .map(|(module, _)| module.as_str());
    let broken = found
        .breaks
        .iter()
        .map(|brk| brk.module.as_str())
        .chain(found.rejected.keys().map(String::as_str));
    [
        first_of(found.unmeasured.iter().map(String::as_str))
            .map(|named| format!("{named} left no record")),
        (found.rewritten == 0).then(|| "the format pass rewrote no file".to_owned()),
        (found.comparable == 0).then(|| "the run compared no module".to_owned()),
        first_of(loader).map(|named| {
            format!(
                "the harness's loader failed {named}, since an import naming the module or a \
                 package holding it raised"
            )
        }),
        first_of(broken).map(|named| format!("the rewrite breaks {named}")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The first of `named` beside how many modules followed it, `None` where
/// there are none.
fn first_of<'a>(mut named: impl Iterator<Item = &'a str>) -> Option<String> {
    let first = named.next()?;
    Some(with_rest(first, named.count(), "module"))
}
