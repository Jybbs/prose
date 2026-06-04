//! The `RuleId` newtype and its parse error.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::registry::KNOWN_IDS;

/// Returned when a string fails to match any registered rule slug.
/// Carries the offending input so callers can surface it verbatim.
#[derive(Debug, Error)]
#[error("unknown rule id `{0}`")]
pub struct ParseRuleIdError(pub String);

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

impl From<&'static str> for RuleId {
    fn from(slug: &'static str) -> Self {
        Self(slug)
    }
}

impl FromStr for RuleId {
    type Err = ParseRuleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        KNOWN_IDS
            .iter()
            .copied()
            .find(|id| id.0 == s)
            .ok_or_else(|| ParseRuleIdError(s.to_owned()))
    }
}

impl Serialize for RuleId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_id_display_and_debug_print_bare_slug() {
        let id = RuleId("align-equals");
        assert_eq!(format!("{id}"), "align-equals");
        assert_eq!(format!("{id:?}"), "align-equals");
    }

    #[test]
    fn rule_id_from_str_rejects_prose_prefixed_slug() {
        let err = "PROSE-align-equals"
            .parse::<RuleId>()
            .expect_err("prefixed form is not the canonical");
        assert_eq!(err.0, "PROSE-align-equals");
    }

    #[test]
    fn rule_id_from_str_rejects_unknown_slug() {
        let err = "not-a-rule"
            .parse::<RuleId>()
            .expect_err("unknown rejected");
        assert_eq!(err.0, "not-a-rule");
    }

    #[test]
    fn rule_id_round_trips_through_display_and_from_str() {
        for id in KNOWN_IDS {
            let parsed: RuleId = id.to_string().parse().expect("known id parses");
            assert_eq!(parsed, *id);
        }
    }
}
