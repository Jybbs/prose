//! Tests over the pipeline, one file per surface they cover.

use std::{
    assert_matches,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::{TextRange, TextSize};

use super::*;
use crate::{
    config::Config,
    diagnostics::{Severity, fired_rules},
    primitives::edit::singleton_groups,
    rules::{
        align_colons::AlignColons, align_equals::AlignEquals,
        alphabetize_siblings::AlphabetizeSiblings,
    },
    testing::{
        FUTURE_LEAD, GroupSentinelRule, PrefixRule, assert_send_sync, breaks_compile, breaks_parse,
        never_settles, notebook, parse, range, rewrites_x_to_y, self_overlapping,
    },
};

mod carry;
mod diagnose;
mod registry;
mod run;
mod run_as_written;
mod second_pass;
mod unsettled;

/// Test-only lint-only rule that returns the range list supplied
/// at construction and never produces edits.
struct LintSentinelRule {
    id: RuleId,
    ranges: Vec<TextRange>,
}

impl Rule for LintSentinelRule {
    fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
        Vec::new()
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn lint(&self, _source: &Source) -> Vec<Diagnostic> {
        let rule = self.id;
        let message = self.message();
        self.ranges
            .iter()
            .map(|&range| Diagnostic::lint(rule, range, message.to_owned()))
            .collect()
    }

    fn message(&self) -> &'static str {
        "lint test rule"
    }
}

/// Test-only lint-only rule that locates `needle` in the source it
/// is handed and emits one lint over it, so its range tracks the
/// buffer the rule actually reads rather than a fixed offset.
struct NeedleLintRule {
    id: RuleId,
    needle: &'static str,
}

impl Rule for NeedleLintRule {
    fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
        Vec::new()
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let start = source.text().find(self.needle).expect("needle is present") as u32;
        let found = range(start, start + self.needle.len() as u32);
        vec![Diagnostic::lint(self.id, found, self.message().to_owned())]
    }

    fn message(&self) -> &'static str {
        "needle lint test rule"
    }
}

/// Test-only rule that records its own id into a shared log and
/// never produces edits.
struct SentinelRule {
    id: RuleId,
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl Rule for SentinelRule {
    fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
        self.log.lock().expect("log mutex").push(self.id.as_str());
        Vec::new()
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn message(&self) -> &'static str {
        "test rule"
    }
}

/// Test-only rule that captures `source.text()` at apply time and
/// returns the edit list supplied at construction.
struct TextCapturingRule {
    edits: Vec<Edit>,
    id: RuleId,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Rule for TextCapturingRule {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        self.seen
            .lock()
            .expect("seen mutex")
            .push(source.text().to_owned());
        singleton_groups(self.edits.clone())
    }

    fn id(&self) -> RuleId {
        self.id
    }

    fn message(&self) -> &'static str {
        "test rule"
    }

    fn preserves_bindings(&self) -> bool {
        false
    }
}

fn registered_slugs(pipeline: &Pipeline) -> Vec<&'static str> {
    pipeline.rule_ids().map(|id| id.as_str()).collect()
}
