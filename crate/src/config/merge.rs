//! Deep-merge of TOML tables, run at the parsed-value layer ahead of
//! deserialization so a partial override layers per knob rather than
//! resetting a whole `#[serde(default)]` struct.

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
        let mut base = table("[rules]\nalign-equals = true\nalphabetize = true\n");
        merge_tables(&mut base, &table("[rules]\nalphabetize = false\n"));

        assert_eq!(
            base,
            table("[rules]\nalign-equals = true\nalphabetize = false\n")
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
        merge_tables(&mut base, &table("[rules]\nalphabetize = false\n"));

        assert_eq!(base, table("[rules]\nalphabetize = false\n"));
    }
}
