//! `prose rules` subcommand: the registered rules in pipeline order.

use std::io::Write;

use serde::Serialize;

use super::args::{RulesArgs, RulesFormat};
use super::exit_status::ExitStatus;
use crate::pipeline::Pipeline;
use crate::rule::message_for_id;

/// One registered rule: its kebab slug, one-based pipeline position,
/// and the imperative the registry carries for it.
#[derive(Serialize)]
struct RuleInfo {
    imperative: &'static str,
    position: usize,
    slug: &'static str,
}

/// Lists every registered rule in pipeline order, as an aligned table
/// or the JSON the docs-site pipeline loader reads.
pub(crate) fn list<W: Write>(args: &RulesArgs, mut stdout: W) -> anyhow::Result<ExitStatus> {
    let rules: Vec<RuleInfo> = Pipeline::known_ids()
        .iter()
        .enumerate()
        .map(|(index, id)| RuleInfo {
            imperative: message_for_id(*id),
            position: index + 1,
            slug: id.as_str(),
        })
        .collect();
    match args.output_format {
        RulesFormat::Json => {
            serde_json::to_writer(&mut stdout, &rules)?;
            writeln!(stdout)?;
        }
        RulesFormat::Table => write_table(&mut stdout, &rules)?,
    }
    Ok(ExitStatus::Clean)
}

fn write_table<W: Write>(mut stdout: W, rules: &[RuleInfo]) -> std::io::Result<()> {
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
    use crate::pipeline::Pipeline;

    fn render(output_format: RulesFormat) -> String {
        let mut out = Vec::new();
        list(&RulesArgs { output_format }, &mut out).expect("rules listing succeeds");
        String::from_utf8(out).expect("utf8 output")
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
    }

    #[test]
    fn table_emits_one_row_per_rule() {
        assert_eq!(
            render(RulesFormat::Table).lines().count(),
            Pipeline::known_ids().len()
        );
    }
}
