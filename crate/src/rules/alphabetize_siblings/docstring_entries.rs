//! The docstring-entry sort of `alphabetize-siblings`, each function's
//! signature-order names read as its mirror key.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::Parameters;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    primitives::{
        docstring::{documented_definitions, entry_carrying_sections, rewrite_docstrings},
        edit::narrowed_replacement,
        orderer::{permute_full, reorder_text},
        params::classify_param,
    },
    source::Source,
};

/// Walks every docstring in `source` and emits one edit per
/// entry-carrying Google-style section whose `name: description`
/// entries are out of order. An entry naming a parameter of the
/// documented signature takes that parameter's position as the rule
/// leaves the signature, and every other entry sinks below them,
/// alphabetized by name. Module and class docstrings carry no
/// signature, so their sections alphabetize throughout. Each edit
/// replaces the section's entries-span with the reordered text.
/// Returns an empty list when no docstring carries a sortable section.
pub(super) fn collect_docstring_entry_edits(source: &Source) -> Vec<Edit> {
    let param_docs: FxHashMap<TextSize, Vec<&str>> = documented_definitions(source)
        .into_iter()
        .filter_map(|(definition, lit)| {
            let function = definition.as_function_def_stmt()?;
            Some((lit.start(), signature_order(&function.parameters)))
        })
        .collect();
    rewrite_docstrings(source, |source, lit, edits| {
        let signature = param_docs.get(&lit.start()).map(Vec::as_slice);
        for section in entry_carrying_sections(source, lit) {
            let (cow, span) = reorder_text(
                source,
                &section.entries,
                |entry| Some(entry_key(entry.name, signature)),
                |_, block| Cow::Borrowed(source.slice(block)),
            );
            let Cow::Owned(text) = cow else {
                continue;
            };
            edits.extend(narrowed_replacement(source, span, text));
        }
    })
    .into_iter()
    .flatten()
    .collect()
}

/// Composite docstring-entry sort key. An entry naming a signature
/// parameter takes that parameter's position, and any other entry
/// sinks below the signature's, alphabetized by name.
fn entry_key<'e>(name: &'e str, signature: Option<&[&str]>) -> (usize, &'e str) {
    signature
        .and_then(|names| names.iter().position(|&n| n == name))
        .map_or((usize::MAX, name), |i| (i, ""))
}

/// Returns the parameter names in the order the rule leaves the
/// signature: positional-only and positional-or-keyword in source
/// order, then `*args`, then the keyword-only block sorted, then
/// `**kwargs`.
fn signature_order(params: &Parameters) -> Vec<&str> {
    let mut names: Vec<&str> = params
        .posonlyargs
        .iter()
        .chain(&params.args)
        .map(|p| p.name().as_str())
        .collect();
    names.extend(params.vararg.as_deref().map(|p| p.name.as_str()));
    let mut order: Vec<usize> = (0..params.kwonlyargs.len()).collect();
    permute_full(&mut order, &params.kwonlyargs, classify_param);
    names.extend(order.iter().map(|&i| params.kwonlyargs[i].name().as_str()));
    names.extend(params.kwarg.as_deref().map(|p| p.name.as_str()));
    names
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, at, parse};

    #[test]
    fn collect_docstring_entry_edits_sinks_stale_entries_below_params() {
        let src = indoc! {"
            class Catalog:
                def update(self, target, source):
                    \"\"\"Apply ``source`` onto ``target``.

                    Args:
                        source: Mapping providing new values.
                        retries: Attempts before giving up.
                        target: Mapping receiving the update.
                    \"\"\"
        "};
        let text = entry_sorted_text(src);
        let pos = |needle: &str| at(&text, needle).start();
        assert!(
            pos("target:") < pos("source:") && pos("source:") < pos("retries:"),
            "parameter entries mirror the signature and the stale entry sinks"
        );
    }

    /// The source with every docstring-entry reorder applied.
    fn entry_sorted_text(src: &str) -> String {
        let source = parse(src);
        let edits = collect_docstring_entry_edits(&source);
        applied_text(&source, edits)
    }

    #[rstest]
    #[case(indoc! {"
        class C:
            def m(self, b, a):
                \"\"\"Summary.

                Args:
                    b: two
                    a: one

                Raises:
                    ValueError: bad
                    KeyError: missing
                \"\"\"
    "})]
    #[case(indoc! {"
        def f(b, a):
            \"\"\"Summary.

            Args:
                b: two
                a: one

            Raises:
                ValueError: bad
                KeyError: missing
            \"\"\"
    "})]
    fn collect_docstring_entry_edits_mirrors_source_order_signature(#[case] src: &str) {
        let text = entry_sorted_text(src);
        let pos = |needle: &str| at(&text, needle).start();
        assert!(
            pos("b: two") < pos("a: one"),
            "parameter entries mirror the un-reordered signature"
        );
        assert!(
            pos("KeyError: missing") < pos("ValueError: bad"),
            "non-parameter entries still sort"
        );
    }

    #[test]
    fn collect_docstring_entry_edits_mirrors_vararg_and_kwarg_positions() {
        let src = indoc! {"
            def f(beta, alpha, *zebra, **apple):
                \"\"\"Summary.

                Args:
                    apple: d
                    zebra: c
                    beta: a
                    alpha: b
                \"\"\"
        "};
        let text = entry_sorted_text(src);
        let pos = |needle: &str| at(&text, needle).start();
        assert!(
            pos("zebra:") < pos("apple:"),
            "the vararg mirrors ahead of the kwarg, both in signature order"
        );
    }
}
