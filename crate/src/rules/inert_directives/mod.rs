//! Reports a `skip` or `ignore` directive that holds nothing out of the
//! run reading it: a skip whose removal leaves every rule it names
//! making the same edits, an ignore whose line carries no diagnostic it
//! silences, and a bracketed id naming no registered rule. A listed id
//! naming a rule the run leaves out spares its directive, and a bare
//! `ignore` silencing diagnostics is reported with the slugs it
//! silences. A report on an `ignore` stands outside that directive's
//! own reach.

use std::iter;

use itertools::Itertools;
use ruff_source_file::OneIndexed;
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    pipeline::Held,
    rules::{Rule, RuleId, render_slugs},
    suppression::{Directive, Named, RuleEntry},
};

#[derive(Debug)]
pub(crate) struct InertDirectives;

impl InertDirectives {
    pub(crate) const MESSAGE: &'static str =
        "remove a `skip` or `ignore` directive that holds nothing out";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) const PRESERVES_TREE: bool = true;

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }

    /// The reports on the `ignore` at `line`, reading `silenced` for the
    /// rules whose diagnostics it held out.
    fn ignore_reports(
        &self,
        held: &Held<'_>,
        line: OneIndexed,
        ignore: &Directive,
        silenced: &[(OneIndexed, RuleId)],
    ) -> Vec<Diagnostic> {
        let here: Vec<RuleId> = silenced
            .iter()
            .filter(|&&(at, _)| at == line)
            .map(|&(_, rule)| rule)
            .collect();
        let mut reports = self.unknown_reports(ignore, "ignore");
        match &ignore.named.entry {
            RuleEntry::All if !here.is_empty() => {
                let slugs: Vec<RuleId> = here.iter().copied().sorted().dedup().collect();
                let narrowed = slugs.iter().join(", ");
                let findings = if here.len() == 1 {
                    "finding"
                } else {
                    "findings"
                };
                reports.push(self.report(
                    ignore,
                    format!(
                        "This bare `ignore` silences {} here, so `# prose: ignore[{narrowed}]` \
                         holds out the same {findings}",
                        render_slugs(&slugs),
                    ),
                ));
            }
            RuleEntry::Specific(rules) if rules.iter().any(|rule| here.contains(rule)) => {}
            _ if judged(&ignore.named, held) => reports.push(self.report(
                ignore,
                "This `ignore` holds no finding out, so removing it leaves the findings unchanged",
            )),
            _ => {}
        }
        reports
    }

    /// One report on `directive`, whose comment it spans.
    fn report(&self, directive: &Directive, message: impl Into<String>) -> Diagnostic {
        Diagnostic::lint(self.id(), directive.range(), message.into())
    }

    /// The reports on the skip at `index` of the source's skips.
    fn skip_reports(&self, held: &Held<'_>, index: usize, skip: &Directive) -> Vec<Diagnostic> {
        let mut reports = self.unknown_reports(skip, "skip");
        if judged(&skip.named, held) && !held.skip_holds(index) {
            reports.push(self.report(
                skip,
                "This `skip` holds no rewrite out, so removing it leaves the rewrite unchanged",
            ));
        }
        reports
    }

    /// One report per distinct bracketed id in `directive` that names no
    /// registered rule, `kind` naming the directive's family in the
    /// message.
    fn unknown_reports(&self, directive: &Directive, kind: &str) -> Vec<Diagnostic> {
        directive
            .named
            .unknown
            .iter()
            .unique()
            .map(|id| self.report(directive, format!("`{id}` in this `{kind}` names no rule")))
            .collect()
    }
}

impl Rule for InertDirectives {
    fn audit(&self, held: &Held<'_>) -> Vec<Diagnostic> {
        let source = held.source();
        let suppression = source.suppression_map();
        let mut silenced = held.silenced().to_vec();
        let mut reports = Vec::new();
        for (index, (_, skip)) in suppression.skips().iter().enumerate() {
            if suppression.intersects(skip) {
                continue;
            }
            let line = source.line_index(skip.start());
            let found = self.skip_reports(held, index, skip);
            if suppression.is_lint_suppressed_at(line, Self::SLUG) {
                silenced.extend(iter::repeat_n((line, Self::SLUG), found.len()));
            } else {
                reports.extend(found);
            }
        }
        for (line, ignore) in suppression.ignores() {
            if !suppression.intersects(ignore) {
                reports.extend(self.ignore_reports(held, *line, ignore, &silenced));
            }
        }
        reports.sort_by_key(Ranged::start);
        reports
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// True where `named` is judged for holding nothing out, meaning it
/// names every rule, or lists only rules the run carries and is not
/// made up of unknown ids alone.
fn judged(named: &Named, held: &Held<'_>) -> bool {
    match &named.entry {
        RuleEntry::All => true,
        RuleEntry::Specific(rules) => {
            rules.iter().all(|&rule| held.selects(rule))
                && (!rules.is_empty() || named.unknown.is_empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        pipeline::Pipeline,
        rules::{
            align_equals::AlignEquals, signature_annotations::SignatureAnnotations,
            unsorted_positionals::UnsortedPositionals,
        },
        source::Source,
        testing::{notebook, parse},
    };

    /// The messages a run carrying `inert-directives` and `companions`
    /// reports over `source`, in source order.
    fn messages(source: &Source, companions: &[RuleId]) -> Vec<String> {
        let selected = [&[InertDirectives::SLUG], companions].concat();
        Pipeline::with_filters(&Config::default(), &selected, &[])
            .diagnose(source)
            .into_iter()
            .filter(|diagnostic| diagnostic.rule == InertDirectives::SLUG)
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[rstest]
    #[case::empty_ignore(
        "x = 1  # prose: ignore[]\n",
        "This `ignore` holds no finding out, so removing it leaves the findings unchanged"
    )]
    #[case::empty_skip(
        "x = 1  # prose: skip[]\n",
        "This `skip` holds no rewrite out, so removing it leaves the rewrite unchanged"
    )]
    #[case::repeated_unknown_id(
        "x = 1  # prose: ignore[typo, typo]\n",
        "`typo` in this `ignore` names no rule"
    )]
    #[case::unknown_id_alone(
        "x = 1  # prose: skip[no-such-rule]\n",
        "`no-such-rule` in this `skip` names no rule"
    )]
    fn a_directive_naming_no_registered_rule_takes_one_report(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(messages(&parse(src), &[]), [expected]);
    }

    #[test]
    fn a_bare_ignore_names_each_rule_it_silences_once() {
        let source = parse("def scale(zeta, alpha):  # prose: ignore\n    return zeta + alpha\n");

        assert_eq!(
            messages(
                &source,
                &[SignatureAnnotations::SLUG, UnsortedPositionals::SLUG]
            ),
            [
                "This bare `ignore` silences `signature-annotations`, `unsorted-positionals` \
                 here, so `# prose: ignore[signature-annotations, unsorted-positionals]` holds \
                 out the same findings"
            ],
        );
    }

    #[test]
    fn a_skip_in_a_notebook_cell_is_judged_against_that_cell() {
        let source = notebook(&[
            "host = 1\nport_number = 2  # fmt: skip",
            "connect(host)  # fmt: skip",
        ]);

        assert_eq!(
            messages(&source, &[AlignEquals::SLUG]),
            ["This `skip` holds no rewrite out, so removing it leaves the rewrite unchanged"],
        );
    }
}
