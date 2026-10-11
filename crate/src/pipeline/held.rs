//! What a run's suppression directives held out of it, the record a
//! rule's audit reads once the lint diagnostics have settled.

use std::cell::OnceCell;

use ruff_diagnostics::Edit;
use ruff_source_file::OneIndexed;

use super::batch::Spliceable;
use crate::{
    rules::{Rule, RuleId},
    source::Source,
};

/// What one run's suppression directives held out of it: the lint
/// diagnostics each `# prose: ignore` silenced, and, read on demand,
/// whether a skip directive changes what the run's rules rewrite.
pub(crate) struct Held<'a> {
    /// Each seat's altering edits over `source`, filled on first read.
    altering: Vec<OnceCell<Vec<Edit>>>,
    rules: &'a [Box<dyn Rule>],
    /// The line and rule of each diagnostic an `ignore` silenced.
    silenced: Vec<(OneIndexed, RuleId)>,
    source: &'a Source,
}

impl<'a> Held<'a> {
    pub(super) fn new(
        source: &'a Source,
        rules: &'a [Box<dyn Rule>],
        silenced: Vec<(OneIndexed, RuleId)>,
    ) -> Self {
        Self {
            altering: vec![OnceCell::new(); rules.len()],
            rules,
            silenced,
            source,
        }
    }

    /// True where the run carries `rule`.
    pub(crate) fn selects(&self, rule: RuleId) -> bool {
        self.rules.iter().any(|carried| carried.id() == rule)
    }

    /// The line and rule of each lint diagnostic an `ignore` silenced,
    /// one entry per diagnostic.
    pub(crate) fn silenced(&self) -> &[(OneIndexed, RuleId)] {
        &self.silenced
    }

    /// True where removing the skip directive at `index` of the
    /// source's skips changes the edits a rule it names makes anywhere
    /// in the source. A held row an aligner drops changes the column its
    /// neighbors pad to, so an edit outside the directive's span counts.
    pub(crate) fn skip_holds(&self, index: usize) -> bool {
        let (_, skip) = &self.source.suppression_map().skips()[index];
        let without = self.source.without_skip(index);
        self.rules
            .iter()
            .zip(&self.altering)
            .filter(|(rule, _)| skip.named.entry.matches(rule.id()))
            .any(|(rule, altering)| {
                *altering.get_or_init(|| Spliceable::altering(&**rule, self.source))
                    != Spliceable::altering(&**rule, &without)
            })
    }

    pub(crate) fn source(&self) -> &'a Source {
        self.source
    }
}
