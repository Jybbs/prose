//! The rule identifier: a registered slug as a copyable handle, its
//! parse off a string, and its serde and display forms.

use std::{
    fmt::{self, Display},
    str::FromStr,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::registry::{KNOWN_IDS, slug_index};

/// Returned when a string fails to match any registered rule slug.
/// Carries the offending input so callers can surface it verbatim.
#[derive(Debug, Error)]
#[error("unknown rule id `{0}`")]
pub struct ParseRuleIdError(String);

/// Stable, parseable rule identifier wrapping a kebab-case slug.
/// Returned by [`Rule::id`] and parsed from CLI / pragma input via
/// [`FromStr`]. The canonical handle in `--select` / `--ignore`,
/// `# prose: ignore[...]`, JSON `"rule"` fields, and `github`
/// annotations.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuleId(pub(super) &'static str);

impl RuleId {
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Debug for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl<'de> Deserialize<'de> for RuleId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[cfg(test)]
impl From<&'static str> for RuleId {
    fn from(slug: &'static str) -> Self {
        Self(slug)
    }
}

impl FromStr for RuleId {
    type Err = ParseRuleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        slug_index(s)
            .map(|i| KNOWN_IDS[i])
            .ok_or_else(|| ParseRuleIdError(s.to_owned()))
    }
}

impl Serialize for RuleId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

/// Renders `rules` as a comma-separated list of backticked slugs.
pub fn render_slugs(rules: &[RuleId]) -> impl Display + '_ {
    rules
        .iter()
        .format_with(", ", |rule, f| f(&format_args!("`{rule}`")))
}

/// Returns `true` when `bytes` is a valid kebab-case slug. Non-empty,
/// starts and ends with a lowercase ASCII letter or digit, contains
/// only lowercase ASCII letters, digits, and dashes, and has no `--`
/// substring.
pub(super) const fn is_valid_slug(bytes: &[u8]) -> bool {
    let mut i = 0;
    let mut prev_was_dash = true;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'-' {
            if prev_was_dash {
                return false;
            }
            prev_was_dash = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_was_dash = false;
        } else {
            return false;
        }
        i += 1;
    }
    !prev_was_dash
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{config::Config, pipeline::Pipeline};

    #[rstest]
    fn is_valid_slug_accepts_canonical_kebab_shapes(
        #[values("a", "a-b", "abc123", "inlinable-bindings")] valid: &str,
    ) {
        assert!(is_valid_slug(valid.as_bytes()));
    }

    #[rstest]
    fn is_valid_slug_rejects_invalid_shapes(
        #[values("", "-foo", "foo-", "a--b", "Foo", "abc!")] invalid: &str,
    ) {
        assert!(!is_valid_slug(invalid.as_bytes()));
    }

    #[test]
    fn rule_id_display_and_debug_print_bare_slug() {
        let id = RuleId("align-equals");
        assert_eq!(format!("{id}"), "align-equals");
        assert_eq!(format!("{id:?}"), "align-equals");
    }

    #[rstest]
    fn rule_id_from_str_rejects_a_retired_slug(
        #[values(
            "alphabetize",
            "blank-lines",
            "call-layout",
            "chain-layout",
            "collection-layout",
            "comment-spacing",
            "docstring-expand",
            "docstring-frame",
            "docstring-wrap",
            "import-layout",
            "legacy-union-syntax",
            "shed-parentheses",
            "signature-layout",
            "strip-align-padding",
            "unused-future-annotations"
        )]
        retired: &str,
    ) {
        let err = retired
            .parse::<RuleId>()
            .expect_err("a retired slug resolves against nothing");
        assert_eq!(err.0, retired);
        assert!(Pipeline::for_rule(retired, &Config::default()).is_none());
    }

    #[rstest]
    fn rule_id_from_str_rejects_an_unregistered_slug(
        #[values("not-a-rule", "PROSE-align-equals")] input: &str,
    ) {
        let err = input
            .parse::<RuleId>()
            .expect_err("unregistered slug is rejected");
        assert_eq!(err.0, input);
    }

    #[test]
    fn rule_id_round_trips_through_display_and_from_str() {
        for id in KNOWN_IDS {
            let parsed: RuleId = id.to_string().parse().expect("known id parses");
            assert_eq!(parsed, *id);
        }
    }
}
