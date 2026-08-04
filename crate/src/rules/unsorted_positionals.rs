//! Flags a run of names the call site binds by position when it sits out
//! of alphabetical order, covering a function's positional-or-keyword
//! parameters, free function and method alike, and the annotated field
//! run of a class whose header generates a positional constructor.
//! Lint-only, emits no edits.
//!
//! A positional-binding-decorated function is skipped whole. `self` /
//! `cls`, the positional-only parameters, a `ClassVar` declaration, and
//! the `KW_ONLY` sentinel merely drop from the run, leaving the rest of
//! it still evaluated.

use ruff_python_ast::{Stmt, StmtClassDef};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{
        constructor::{classify_field, keyword_field_start},
        params::{params_unsorted, pins_positional_params},
        range::blocks_span,
        walk::filter_map_over_stmts,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct UnsortedPositionals;

impl UnsortedPositionals {
    pub(crate) const MESSAGE: &'static str = "Positional run is out of alphabetical order. Reordering rebinds every positional call site, so apply it by hand where every caller binds by keyword";

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for UnsortedPositionals {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let message = self.message();
        let rule = self.id();
        filter_map_over_stmts(&source.ast().body, |stmt| {
            unsorted_run(stmt).map(|range| Diagnostic::lint(rule, range, message.to_owned()))
        })
    }
}

/// The extent of the field run a class's generated constructor binds by
/// position, returned only where sorting that run would change its
/// order. `None` where the header generates no positional constructor
/// and where the run already reads in order.
fn unsorted_field_run(class: &StmtClassDef) -> Option<TextRange> {
    let binds_by_name_from = keyword_field_start(class);
    let (ranges, keys): (Vec<TextRange>, Vec<(u8, &str)>) = class
        .body
        .iter()
        .take_while(|stmt| stmt.start() < binds_by_name_from)
        .filter_map(|stmt| classify_field(stmt).map(|key| (stmt.range(), key)))
        .unzip();
    (!keys.is_sorted()).then(|| blocks_span(&ranges))
}

/// The extent of the positional run `stmt` declares, returned only where
/// sorting that run would change its order. `None` for a statement
/// declaring no run, for a decorator-bound signature, and for a run
/// already in order.
fn unsorted_run(stmt: &Stmt) -> Option<TextRange> {
    match stmt {
        Stmt::ClassDef(class) => unsorted_field_run(class),
        Stmt::FunctionDef(f) => (!pins_positional_params(f) && params_unsorted(&f.parameters))
            .then(|| f.parameters.range()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::{diagnostics::Severity, testing::parse};

    /// The source slices the rule reports against `source`.
    fn flagged(source: &Source) -> Vec<&str> {
        UnsortedPositionals
            .lint(source)
            .iter()
            .map(|d| &source.text()[d.range])
            .collect()
    }

    #[test]
    fn flags_a_free_function_and_a_method_in_one_module() {
        let src = indoc! {"
            class Catalog:
                def update(self, target, source): pass


            def free(target, source): pass
        "};
        let source = parse(src);
        assert_eq!(
            flagged(&source),
            vec!["(self, target, source)", "(target, source)"],
        );
    }

    #[test]
    fn flags_a_function_nested_in_a_method() {
        let src = indoc! {"
            class C:
                def m(self):
                    def inner(target, source): pass
        "};
        let source = parse(src);
        assert_eq!(flagged(&source), vec!["(target, source)"]);
    }

    #[test]
    fn flags_an_unsorted_free_function() {
        let source = parse("def merge(target, source): pass\n");
        let diagnostics = UnsortedPositionals.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");
        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert_eq!(&source.text()[only.range], "(target, source)");
    }

    #[test]
    fn flags_the_field_run_of_a_generated_constructor() {
        let src = indoc! {"
            @dataclass
            class C:
                width: int
                height: int
        "};
        let source = parse(src);
        assert_eq!(flagged(&source), vec!["width: int\n    height: int"]);
    }

    #[test]
    fn leaves_a_sorted_free_function_quiet() {
        let source = parse("def merge(source, target): pass\n");
        assert!(flagged(&source).is_empty());
    }

    #[test]
    fn respects_the_required_before_optional_partition() {
        let source = parse("def f(source, target, fallback=None): pass\n");
        assert!(flagged(&source).is_empty());
    }
}
