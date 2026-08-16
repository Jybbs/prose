//! Narrows a run whose output a second pass would change down to the
//! smallest rule subset still reproducing it.
//!
//! The search runs each rule alone first and each rule pair after,
//! returning the first subset whose own output a second pass changes. A
//! pair is built in registration order, the order
//! [`Pipeline::with_filters`] seats whichever subset it is handed.

use itertools::Itertools;

use crate::{pipeline::Pipeline, rule::RuleId, source::Source};

/// `build` assembles the pipeline for one subset. `None` once neither a
/// single rule nor a pair reproduces on this source.
pub(super) fn reproducing_subset(
    candidates: &[RuleId],
    text: &str,
    build: impl Fn(&[RuleId]) -> Pipeline,
) -> Option<Vec<RuleId>> {
    let singles = candidates.iter().map(|&rule| vec![rule]);
    let pairs = candidates
        .iter()
        .array_combinations()
        .map(|[&earlier, &later]| vec![earlier, later]);
    singles
        .chain(pairs)
        .find(|subset| leaves_an_edit(&build(subset), text))
}

/// A `text` that does not parse and a run a rule's output is rejected
/// on both answer false.
fn leaves_an_edit(pipeline: &Pipeline, text: &str) -> bool {
    let Ok(source) = text.parse::<Source>() else {
        return false;
    };
    let Ok((formatted, _)) = pipeline.run(source) else {
        return false;
    };
    !pipeline.unsettled(&formatted).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rule::Rule,
        testing::{GroupSentinelRule, breaks_parse, never_settles},
    };

    const SOURCE: &str = "x = 1\n";

    /// A pipeline over the sentinel rules `subset` names, each slug in
    /// `settles` emitting no edit and every other one never settling.
    fn sentinels(subset: &[RuleId], settles: &[&str]) -> Pipeline {
        let rules = subset
            .iter()
            .map(|&rule| -> Box<dyn Rule> {
                if settles.contains(&rule.as_str()) {
                    Box::new(GroupSentinelRule {
                        groups: Vec::new(),
                        id: rule,
                    })
                } else {
                    Box::new(never_settles(rule.as_str()))
                }
            })
            .collect();
        Pipeline::from_rules(rules)
    }

    #[test]
    fn reproducing_subset_falls_through_when_every_subset_settles() {
        let candidates = [RuleId::from("calm-a"), RuleId::from("calm-b")];

        let found = reproducing_subset(&candidates, SOURCE, |subset| {
            sentinels(subset, &["calm-a", "calm-b"])
        });

        assert_eq!(found, None);
    }

    #[test]
    fn reproducing_subset_is_none_for_an_empty_candidate_list() {
        let found = reproducing_subset(&[], SOURCE, |subset| sentinels(subset, &[]));

        assert_eq!(found, None);
    }

    #[test]
    fn reproducing_subset_names_a_pair_when_no_rule_reproduces_alone() {
        let candidates = [
            RuleId::from("calm-a"),
            RuleId::from("calm-b"),
            RuleId::from("calm-c"),
        ];
        let together = [RuleId::from("calm-b"), RuleId::from("calm-c")];

        let found = reproducing_subset(&candidates, SOURCE, |subset| {
            if subset == together {
                sentinels(subset, &["calm-b"])
            } else {
                sentinels(subset, &["calm-a", "calm-b", "calm-c"])
            }
        });

        assert_eq!(found, Some(together.to_vec()));
    }

    #[test]
    fn reproducing_subset_names_one_rule_over_the_pair_holding_it() {
        let candidates = [RuleId::from("calm-a"), RuleId::from("widener")];

        let found =
            reproducing_subset(&candidates, SOURCE, |subset| sentinels(subset, &["calm-a"]));

        assert_eq!(found, Some(vec![RuleId::from("widener")]));
    }

    #[test]
    fn reproducing_subset_passes_over_a_subset_whose_run_is_rejected() {
        let candidates = [RuleId::from("breaks-parse")];

        let found = reproducing_subset(&candidates, SOURCE, |_| {
            Pipeline::from_rules(vec![Box::new(breaks_parse())])
        });

        assert_eq!(found, None);
    }

    #[test]
    fn reproducing_subset_passes_over_a_source_that_does_not_parse() {
        let candidates = [RuleId::from("widener")];

        let found = reproducing_subset(&candidates, "def foo(", |subset| sentinels(subset, &[]));

        assert_eq!(found, None);
    }
}
