//! Deep-merge and deep-diff of TOML tables, run at the parsed-value
//! layer ahead of deserialization so a partial override layers per knob
//! rather than resetting a whole `#[serde(default)]` struct.

/// Recursively merges `overlay` into `base`. A key both carry as a table
/// merges field by field, so an override wins the knobs it sets and
/// leaves the rest. Any other overlay value replaces `base`'s.
pub(super) fn merge_tables(base: &mut toml::Table, overlay: &toml::Table) {
    for (key, value) in overlay {
        match (base.get_mut(key), value) {
            (Some(toml::Value::Table(into)), toml::Value::Table(from)) => merge_tables(into, from),
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Drops every entry of `table` matching `defaults`, recursing into a
/// key both carry as a table and dropping that sub-table once it empties.
/// A key `defaults` does not carry stays.
pub(super) fn without_defaults(table: &mut toml::Table, defaults: &toml::Table) {
    table.retain(|key, value| match (value, defaults.get(key)) {
        (toml::Value::Table(sub), Some(toml::Value::Table(base))) => {
            without_defaults(sub, base);
            !sub.is_empty()
        }
        (value, Some(base)) => value != base,
        (_, None) => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(toml: &str) -> toml::Table {
        toml.parse().expect("parses")
    }

    #[test]
    fn disjoint_keys_accumulate() {
        let mut base = table("a = 1\n");
        merge_tables(&mut base, &table("b = 2\n"));

        assert_eq!(base, table("a = 1\nb = 2\n"));
    }

    #[test]
    fn nested_tables_merge_field_by_field() {
        let mut base = table("[rules]\nalign-equals = true\nalphabetize-siblings = true\n");
        merge_tables(&mut base, &table("[rules]\nalphabetize-siblings = false\n"));

        assert_eq!(
            base,
            table("[rules]\nalign-equals = true\nalphabetize-siblings = false\n")
        );
    }

    #[test]
    fn overlay_scalar_replaces_base_scalar() {
        let mut base = table("code-line-length = 88\n");
        merge_tables(&mut base, &table("code-line-length = 120\n"));

        assert_eq!(base, table("code-line-length = 120\n"));
    }

    #[test]
    fn overlay_table_replaces_base_scalar() {
        let mut base = table("rules = false\n");
        merge_tables(&mut base, &table("[rules]\nalphabetize-siblings = false\n"));

        assert_eq!(base, table("[rules]\nalphabetize-siblings = false\n"));
    }

    #[test]
    fn without_defaults_drops_a_sub_table_that_empties() {
        let mut set = table("[rules]\nalign-equals = true\n[imports]\nfirst-party = [\"app\"]\n");
        without_defaults(
            &mut set,
            &table("[rules]\nalign-equals = true\n[imports]\nfirst-party = []\n"),
        );

        assert_eq!(set, table("[imports]\nfirst-party = [\"app\"]\n"));
    }

    #[test]
    fn without_defaults_empties_a_table_matching_the_defaults() {
        let mut set = table("code-line-length = 88\n[rules]\nalign-equals = true\n");
        let defaults = set.clone();
        without_defaults(&mut set, &defaults);

        assert_eq!(set, toml::Table::new());
    }

    #[test]
    fn without_defaults_keeps_a_key_the_defaults_lack() {
        let mut set = table("target-version = \"3.14\"\n");
        without_defaults(&mut set, &table("code-line-length = 88\n"));

        assert_eq!(set, table("target-version = \"3.14\"\n"));
    }

    #[test]
    fn without_defaults_keeps_only_the_keys_that_differ() {
        let mut set = table("code-line-length = 100\n[rules]\nalign-equals = false\n");
        without_defaults(
            &mut set,
            &table("code-line-length = 88\n[rules]\nalign-equals = true\n"),
        );

        assert_eq!(
            set,
            table("code-line-length = 100\n[rules]\nalign-equals = false\n")
        );
    }
}
