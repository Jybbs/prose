//! Strips padding that aligns with nothing. On a colon context with no
//! column to align to it clears the pre-colon gap and collapses the
//! post-colon gap to one space, and it clears the space just inside a
//! bracket delimiter. Runs after the alignment rules in
//! `Pipeline::with_defaults` so it sees their output, and the edits it
//! emits are the ones `primitives::padding` lists for a rule measuring
//! a row ahead of it.

use ruff_diagnostics::Edit;

use crate::{
    config::Config,
    primitives::{edit::singleton_groups, padding::Stranding},
    rules::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct StripStrandedPadding {
    stranding: Stranding,
}

impl StripStrandedPadding {
    pub(crate) const MESSAGE: &'static str = "drop padding that lines up with nothing";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            stranding: config.stranded_padding(),
        }
    }
}

impl Rule for StripStrandedPadding {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(source.stranded_padding(self.stranding).into_owned())
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}
