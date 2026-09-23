//! The docstring-entry sort of `alphabetize-siblings`, the
//! signature-order names of each function and the fields that hold their
//! slots in each class read as its mirror key.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Parameters, Stmt, StmtClassDef};
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    primitives::{
        binding::single_name_assignment,
        comments::class_keeps_order,
        constructor::{classify_field, keyword_field_start},
        docstring::{documented_definitions, entry_carrying_sections, rewrite_docstrings},
        edit::narrowed_replacement,
        orderer::{permute_full, reorder_text},
        params::classify_param,
    },
    source::Source,
};

/// The names whose order the entries of one docstring follow, beside
/// whether an entry naming none of them holds its slot rather than
/// sinking below them, alphabetized.
struct Mirror<'a> {
    holds_rest: bool,
    names: Vec<&'a str>,
}

/// Walks every docstring in `source` and emits one edit per
/// entry-carrying Google-style section whose `name: description`
/// entries are out of order, each edit replacing the section's entries
/// span with the reordered text. An entry naming a parameter of the
/// documented signature takes that parameter's position as the rule
/// leaves the signature, and every other entry sinks below them,
/// alphabetized by name. A class docstring mirrors the fields that hold
/// their slots per [`class_mirror`], whereas the sections of every other
/// module and class docstring alphabetize throughout.
pub(super) fn collect_docstring_entry_edits(source: &Source) -> Vec<Edit> {
    let mirrors: FxHashMap<TextSize, Mirror<'_>> = documented_definitions(source)
        .into_iter()
        .filter_map(|(definition, lit)| {
            let mirror = match definition {
                Stmt::ClassDef(class) => class_mirror(source, class)?,
                Stmt::FunctionDef(function) => Mirror {
                    holds_rest: false,
                    names: signature_order(&function.parameters),
                },
                _ => return None,
            };
            Some((lit.start(), mirror))
        })
        .collect();
    rewrite_docstrings(source, |source, lit, edits| {
        let mirror = mirrors.get(&lit.start());
        for section in entry_carrying_sections(source, lit) {
            let (cow, span) = reorder_text(
                source,
                &section.entries,
                |entry| entry_key(entry.name, mirror),
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

/// Returns the fields of `class` that hold their slots, in the order
/// written, every entry naming none of them holding its slot too. Under a
/// `# prose: keep` header that is every single-name assignment, whereas a
/// class whose header generates its constructor holds the fields bound
/// by position. `None` where no field holds its slot.
fn class_mirror<'a>(source: &Source, class: &'a StmtClassDef) -> Option<Mirror<'a>> {
    let names: Vec<&str> = if class_keeps_order(source, class) {
        class
            .body
            .iter()
            .filter_map(single_name_assignment)
            .map(|(name, _)| name.id.as_str())
            .collect()
    } else {
        let keyword_start = keyword_field_start(class);
        class
            .body
            .iter()
            .take_while(|stmt| stmt.start() < keyword_start)
            .filter_map(classify_field)
            .map(|(_, name)| name)
            .collect()
    };
    (!names.is_empty()).then_some(Mirror {
        holds_rest: true,
        names,
    })
}

/// Composite docstring-entry sort key. An entry naming one of the
/// `mirror` names takes that name's position, and any other entry sinks
/// below them, alphabetized by name, or holds its slot as `None` where
/// the mirror holds the rest.
fn entry_key<'e>(name: &'e str, mirror: Option<&Mirror<'_>>) -> Option<(usize, &'e str)> {
    match mirror.and_then(|mirror| mirror.names.iter().position(|&n| n == name)) {
        Some(position) => Some((position, "")),
        None if mirror.is_some_and(|mirror| mirror.holds_rest) => None,
        None => Some((usize::MAX, name)),
    }
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

    #[rstest]
    #[case("class Station:  # prose: keep", ["zone:", "station:", "latitude:"])]
    #[case("@dataclass\nclass Station:", ["zone:", "station:", "latitude:"])]
    #[case("class Station:", ["latitude:", "station:", "zone:"])]
    fn collect_docstring_entry_edits_mirrors_the_fields_holding_their_slots(
        #[case] header: &str,
        #[case] expected: [&str; 3],
    ) {
        let src = indoc! {"
            HEADER
                \"\"\"Summary.

                Attributes:
                    zone: Stale entry.
                    latitude: Degrees north.
                    station: Identifier.
                \"\"\"

                station: str
                latitude: float
        "}
        .replace("HEADER", header);
        let text = entry_sorted_text(&src);
        let pos = |needle: &str| at(&text, needle).start();
        assert!(
            pos(expected[0]) < pos(expected[1]) && pos(expected[1]) < pos(expected[2]),
            "the entries read {expected:?}"
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
