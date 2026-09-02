//! The bug-report URL an unstable rewrite pre-fills.
//!
//! Each query parameter is keyed by the matching field `id` in
//! `.github/ISSUE_TEMPLATE/unstable-output.yml`. A field whose value
//! would carry the URL past its budget is left out and the reporter
//! pastes it.

use fluent_uri::pct_enc::{
    EString,
    encoder::{Data, Query},
};
use std::slice;

use super::UnstableRewrite;
use crate::rules::render_slugs;

const TEMPLATE: &str = "unstable-output.yml";

/// The budget every report URL stays under, terminal-printed and
/// client-opened alike, so what travels toward a public tracker is the
/// same bounded content the notice prints. A field too long for it
/// drops, which on a real module is the source and both passes, and
/// the form asks the reporter for the file instead.
const BUDGET: usize = 1200;

pub(crate) fn report_url(rewrite: &UnstableRewrite, original: &str) -> String {
    build(rewrite, original, BUDGET)
}

/// The fields run smallest-first, so an oversized one drops while every
/// field after it still lands.
fn build(rewrite: &UnstableRewrite, original: &str, budget: usize) -> String {
    let slugs = rewrite.slugs();
    let title = match rewrite.rules.as_slice() {
        [first, rest @ ..] if rest.len() > 1 => format!(
            "Unstable output from {} and {} more rules",
            render_slugs(slice::from_ref(first)),
            rest.len(),
        ),
        rules => format!("Unstable output from {}", render_slugs(rules)),
    };
    let fields = [
        ("title", title.as_str()),
        ("version", env!("CARGO_PKG_VERSION")),
        ("rules", slugs.as_str()),
        ("config", rewrite.config_toml.as_str()),
        ("source", original),
        ("first-pass", rewrite.first.as_str()),
        ("second-pass", rewrite.second.as_str()),
    ];
    fields.iter().fold(
        format!(
            "{}/issues/new?template={TEMPLATE}",
            env!("CARGO_PKG_REPOSITORY")
        ),
        |url, (id, value)| {
            let field = format!("&{id}={}", encoded(value));
            if url.len() + field.len() <= budget {
                url + &field
            } else {
                url
            }
        },
    )
}

/// `value` percent-encoded down to the unreserved characters, so no
/// `&` or `=` it carries reads as a parameter delimiter.
fn encoded(value: &str) -> EString<Query> {
    let mut buf = EString::<Query>::new();
    buf.encode_str::<Data>(value);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RuleId;

    fn rewrite(first: &str, second: &str, config_toml: &str) -> UnstableRewrite {
        UnstableRewrite {
            config_toml: config_toml.to_owned(),
            first: first.to_owned(),
            rules: vec![RuleId::from("align-equals")],
            second: second.to_owned(),
        }
    }

    #[test]
    fn report_url_drops_a_field_that_would_overflow_the_budget() {
        let huge = "x".repeat(BUDGET);

        let url = report_url(&rewrite(&huge, "b = 2\n", ""), "a = 1\n");

        assert!(url.len() <= BUDGET, "budget overrun: {}", url.len());
        assert!(!url.contains("&first-pass="), "oversized field survived");
        assert!(url.contains("&second-pass=b%20%3D%202%0A"));
    }

    #[test]
    fn report_url_encodes_every_delimiter_a_value_carries() {
        let url = report_url(&rewrite("a = 1\n", "b = 2\n", ""), "q = a & b\n");

        assert!(url.contains("&source=q%20%3D%20a%20%26%20b%0A"));
    }

    #[test]
    fn report_url_fills_every_field_the_form_declares() {
        const FORM: &str = include_str!("../../../.github/ISSUE_TEMPLATE/unstable-output.yml");

        let declared: Vec<&str> = FORM
            .lines()
            .filter_map(|line| line.trim().strip_prefix("id")?.split_once(':'))
            .map(|(_, value)| value.trim())
            .collect();
        let url = report_url(&rewrite("a = 1\n", "b = 2\n", "x = 1\n"), "a = 1\n");

        assert!(!declared.is_empty(), "no field ids read from the template");
        for id in declared {
            // `notes` is the one field the reporter fills by hand.
            assert_eq!(
                url.contains(&format!("&{id}=")),
                id != "notes",
                "field `{id}` disagrees with the URL",
            );
        }
    }

    #[test]
    fn report_url_names_the_template_and_every_field() {
        let url = report_url(
            &rewrite("a = 1\n", "b = 2\n", "code-line-length = 88\n"),
            "a = 1\n",
        );

        assert!(url.starts_with(&format!(
            "{}/issues/new?template={TEMPLATE}",
            env!("CARGO_PKG_REPOSITORY"),
        )));
        assert!(url.contains("&title=Unstable%20output%20from%20%60align-equals%60"));
        assert!(url.contains(&format!("&version={}", env!("CARGO_PKG_VERSION"))));
        assert!(url.contains("&rules=align-equals"));
        assert!(url.contains("&config=code-line-length%20%3D%2088%0A"));
        assert!(url.contains("&first-pass=a%20%3D%201%0A"));
    }

    #[test]
    fn report_url_titles_a_pair_with_both_slugs() {
        let mut pair = rewrite("a = 1\n", "b = 2\n", "");
        pair.rules.push(RuleId::from("align-colons"));

        let url = report_url(&pair, "a = 1\n");

        assert!(url.contains(
            "&title=Unstable%20output%20from%20%60align-equals%60%2C%20%60align-colons%60"
        ));
        assert!(url.contains("&rules=align-equals%2Calign-colons"));
    }

    #[test]
    fn report_url_sheds_the_bulky_fields_under_its_budget() {
        let bulky = "x".repeat(2000);
        let rewrite = rewrite(&bulky, &bulky, "code-line-length = 100\n");

        let url = report_url(&rewrite, &bulky);

        assert!(url.len() <= BUDGET, "budget overrun: {}", url.len());
        assert!(url.contains("&rules=align-equals"));
        assert!(url.contains("&config=code-line-length%20%3D%20100%0A"));
        assert!(!url.contains("&source="), "{url}");
        assert!(!url.contains("&first-pass="), "{url}");
    }
}
