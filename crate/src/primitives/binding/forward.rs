//! Carries a binding table across a reparse, moving every offset and
//! range it holds to where the woven text carries it.

use std::{mem, sync::OnceLock};

use ruff_diagnostics::SourceMap;
use ruff_text_size::TextSize;

use super::{BindingAnalysis, UnpackKind};
use crate::primitives::edit::{forward_range, forward_start};

impl BindingAnalysis {
    /// This table over the woven text `map` describes, every write and
    /// read offset, definition start, and value range moved to where
    /// that text carries it. `None` where an edit in `map` replaced a
    /// token the table names.
    pub(crate) fn forwarded(mut self, map: &SourceMap) -> Option<Self> {
        for binding in &mut self.bindings {
            forward_each(&mut binding.read_offsets, map)?;
            forward_each(&mut binding.write_offsets, map)?;
            if let Some(first) = &mut binding.first_unconditional_write {
                *first = forward_start(*first, map)?;
            }
        }
        for writes in self.global_writes.values_mut() {
            forward_each(writes, map)?;
        }
        for kind in self.unpack_targets.values_mut() {
            if let UnpackKind::Suggested(value, _) = kind {
                *value = forward_range(*value, map)?;
            }
        }
        self.assignment_values = mem::take(&mut self.assignment_values)
            .into_iter()
            .map(|(name, value)| Some((forward_start(name, map)?, forward_range(value, map)?)))
            .collect::<Option<_>>()?;
        self.function_scope_at = mem::take(&mut self.function_scope_at)
            .into_iter()
            .map(|(start, scope)| Some((forward_start(start, map)?, scope)))
            .collect::<Option<_>>()?;
        self.module_reads = OnceLock::new();
        Some(self)
    }
}

/// Moves each of `offsets`, every one a token's first byte, to where
/// the woven text `map` describes carries it, `None` where an edit in
/// `map` replaced one of the tokens.
fn forward_each(offsets: &mut [TextSize], map: &SourceMap) -> Option<()> {
    offsets.iter_mut().try_for_each(|offset| {
        *offset = forward_start(*offset, map)?;
        Some(())
    })
}

#[cfg(test)]
mod tests {
    use ruff_diagnostics::Edit;

    use super::*;
    use crate::testing::{parse, range, woven};

    /// The table over `before` moved through the map of `edits`, beside
    /// the table a fresh walk over the woven text builds.
    fn carried_and_fresh(
        before: &str,
        edits: Vec<Edit>,
    ) -> (Option<BindingAnalysis>, BindingAnalysis) {
        let source = parse(before);
        let (after, map) = woven(source.text(), edits);
        let carried = BindingAnalysis::new(source.ast()).forwarded(&map);
        (carried, BindingAnalysis::new(parse(&after).ast()))
    }

    #[test]
    fn forwarded_answers_none_where_an_edit_replaced_a_named_token() {
        let (carried, _) = carried_and_fresh(
            "x = 1\ny = x\n",
            vec![Edit::range_replacement("(x)".to_owned(), range(10, 11))],
        );

        assert!(carried.is_none());
    }

    #[test]
    fn forwarded_matches_a_fresh_walk_over_the_woven_text() {
        let before = "import os\n\n\nglobal_var = 1\n\n\ndef f(a, b):\n    global global_var\n    global_var = a\n    c, d = pair\n    return os.path.join(c, d, b)\n";
        let (carried, fresh) = carried_and_fresh(
            before,
            vec![
                Edit::range_deletion(range(10, 12)),
                Edit::insertion("    pass\n".to_owned(), TextSize::of(before)),
            ],
        );

        assert_eq!(carried.expect("every token survives"), fresh);
    }

    #[test]
    fn forwarded_rebuilds_the_module_reads_a_first_pass_filled() {
        let before = "import os\n\n\nvalue = os.getcwd()\n";
        let source = parse(before);
        let (after, map) = woven(source.text(), vec![Edit::range_deletion(range(10, 12))]);
        let table = BindingAnalysis::new(source.ast());
        table.module_names_read_within(&[range(0, before.len() as u32)]);

        let carried = table.forwarded(&map).expect("every token survives");
        let fresh = BindingAnalysis::new(parse(&after).ast());

        assert_eq!(carried, fresh);
        let moved = [range(0, after.len() as u32)];
        assert_eq!(
            carried.module_names_read_within(&moved),
            fresh.module_names_read_within(&moved),
        );
    }
}
