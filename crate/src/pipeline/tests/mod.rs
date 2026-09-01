//! Pipeline tests, one file per surface they cover, with the sentinel
//! rules and helpers they share held here.

use std::{
    assert_matches,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use rstest::rstest;
use ruff_diagnostics::Edit;
use ruff_text_size::{TextLen, TextRange, TextSize};

use super::*;
use crate::{
    config::Config,
    diagnostics::Severity,
    primitives::edit::singleton_groups,
    rules::{
        align_colons::AlignColons, align_equals::AlignEquals,
        alphabetize_siblings::AlphabetizeSiblings,
    },
    testing::{
        FUTURE_LEAD, GroupSentinelRule, PrefixRule, assert_send_sync, breaks_compile, breaks_parse,
        never_settles, notebook, parse, range, replacement, self_overlapping,
    },
};

mod as_written;
mod batch;
mod diagnose;
mod registry;
mod run;
mod settle;
mod subsets;

/// Test-only lint-only rule that returns the range list supplied
/// at construction and never produces edits.
#[derive(Debug)]
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
#[derive(Debug)]
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
#[derive(Debug)]
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
#[derive(Debug)]
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
}

/// A text-capturing sentinel under `slug` holding `edits`, logging
/// every buffer it reads into `seen`.
fn capturing(
    seen: &Arc<Mutex<Vec<String>>>,
    slug: &'static str,
    edits: Vec<Edit>,
) -> Box<dyn Rule> {
    Box::new(TextCapturingRule {
        edits,
        id: RuleId::from(slug),
        seen: Arc::clone(seen),
    })
}

/// The buffers a capture log holds, in the order the rules read them.
fn captured(seen: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    seen.lock().expect("seen mutex").clone()
}

fn registered_slugs(pipeline: &Pipeline) -> Vec<&'static str> {
    pipeline.rule_ids().map(|id| id.as_str()).collect()
}
