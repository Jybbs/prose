//! `prose rules` subcommand: the registered rules in pipeline order.

use std::io::{self, Write};

use serde::Serialize;

use super::{
    args::{RulesArgs, RulesFormat},
    emit::write_json_line,
    exit_status::ExitStatus,
};
use crate::{
    pipeline::Pipeline,
    rule::{dependencies_of, message_for_id},
};

/// One registered rule, carrying its one-based pipeline position and
/// the slugs it must run behind.
#[derive(Serialize)]
struct RuleInfo {
    after: &'static [&'static str],
    imperative: &'static str,
    position: usize,
    slug: &'static str,
}

/// Lists every registered rule in pipeline order, as an aligned table
/// or a JSON array.
pub(crate) fn list<W: Write>(args: &RulesArgs, mut stdout: W) -> anyhow::Result<ExitStatus> {
    let rules: Vec<RuleInfo> = Pipeline::known_ids()
        .iter()
        .enumerate()
        .map(|(index, id)| RuleInfo {
            after: dependencies_of(id.as_str()),
            imperative: message_for_id(*id),
            position: index + 1,
            slug: id.as_str(),
        })
        .collect();
    match args.output_format {
        RulesFormat::Json => {
            write_json_line(&mut stdout, &rules)?;
        }
        RulesFormat::Table => write_table(&mut stdout, &rules)?,
    }
    Ok(ExitStatus::Clean)
}

fn write_table<W: Write>(mut stdout: W, rules: &[RuleInfo]) -> io::Result<()> {
    let slug_width = rules.iter().map(|rule| rule.slug.len()).max().unwrap_or(0);
    let pos_width = rules.len().to_string().len();
    for rule in rules {
        writeln!(
            stdout,
            "{:>pos_width$}  {:slug_width$}  {}",
            rule.position, rule.slug, rule.imperative,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(output_format: RulesFormat) -> String {
        crate::testing::rendered(|out| {
            list(&RulesArgs { output_format }, out).expect("rules listing succeeds");
        })
    }

    #[test]
    fn json_carries_each_rule_dependency_list() {
        let rules: Vec<serde_json::Value> =
            serde_json::from_str(&render(RulesFormat::Json)).expect("valid JSON array");
        for rule in &rules {
            let slug = rule["slug"].as_str().expect("slug renders as a string");
            let after: Vec<&str> = rule["after"]
                .as_array()
                .expect("after renders as an array")
                .iter()
                .map(|name| name.as_str().expect("dependency renders as a string"))
                .collect();
            assert_eq!(after, dependencies_of(slug));
        }
    }

    #[test]
    fn json_lists_every_registered_rule_in_pipeline_order() {
        let rules: Vec<serde_json::Value> =
            serde_json::from_str(&render(RulesFormat::Json)).expect("valid JSON array");
        assert_eq!(rules.len(), Pipeline::known_ids().len());
        assert_eq!(rules[0]["position"].as_u64(), Some(1));
        assert_eq!(
            rules[0]["slug"].as_str(),
            Some(Pipeline::known_ids()[0].as_str())
        );
        assert!(
            rules[0]["imperative"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
            "every rule carries a non-empty imperative: {:?}",
            rules[0],
        );
    }

    #[test]
    fn table_emits_one_row_per_rule() {
        assert_eq!(
            render(RulesFormat::Table).lines().count(),
            Pipeline::known_ids().len()
        );
    }

    #[test]
    fn table_right_aligns_single_digit_positions() {
        let table = render(RulesFormat::Table);
        let first = table.lines().next().expect("registry is non-empty");
        assert!(
            first.starts_with(' '),
            "position 1 right-aligns under the widest position: {first:?}",
        );
    }
}
