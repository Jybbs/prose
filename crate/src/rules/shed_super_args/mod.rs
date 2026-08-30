//! Sheds the arguments of a parameterized `super(C, self)` call,
//! leaving the bare `super()`, where the first argument names the one
//! enclosing class and the second the enclosing callable's first
//! positional parameter. A comprehension, a callable taking no
//! positional parameter, a `@dataclass(slots=True)` class, a comment
//! inside the argument list, a scope binding the class name, and a
//! module binding `super` or `__class__` each hold the arguments, and
//! a continuation row the deletion would strand re-seats by the
//! columns the span took unless a row-spanning string freezes it.

use ruff_diagnostics::Edit;
use ruff_python_ast::visitor::Visitor;
use ruff_text_size::TextSize;

use crate::{
    config::Config,
    primitives::{
        binding::BindingAnalysis, decorator::is_slots_dataclass, edit::insert_edit,
        inline::indent_width, params::first_positional, reseat::push_reseat_edits,
        travel::frozen_rows, walk::walk_stmt,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod walk;

use walk::Walker;

pub(crate) struct ShedSuperArgs;

impl ShedSuperArgs {
    pub(crate) const MESSAGE: &'static str =
        "shed the arguments a parameterized `super()` call restates";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for ShedSuperArgs {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let analysis = source.binding_analysis();
        if analysis.binds_name("__class__") || analysis.binds_name("super") {
            return Vec::new();
        }
        let mut walker = Walker {
            analysis,
            classes: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
            source,
            statement: TextSize::default(),
        };
        walker.visit_body(&source.ast().body);
        walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::aligned_continuation(
        "class C:\n    def m(self, a, b):\n        return super(C, self).m(a,\n                                b)\n",
        Some(7)
    )]
    #[case::hanging_continuation(
        "class C:\n    def m(self, a, b):\n        return super(C, self).m(\n            a, b)\n",
        None
    )]
    #[case::enclosing_closer(
        "class C:\n    def m(self, a, b):\n        return zz(\n            super(C, self).m(a,\n                             b)\n        )\n",
        Some(7)
    )]
    #[case::spread_span(
        "class C:\n    def m(self, a, b):\n        return super(\n            C,\n            self\n        ).m(a,\n            b)\n",
        None
    )]
    fn a_continuation_re_seats_by_the_columns_the_span_took(
        #[case] src: &str,
        #[case] re_seated: Option<u32>,
    ) {
        let source = parse(src);
        let groups = ShedSuperArgs.apply(&source);
        let indents: Vec<u32> = groups[0]
            .iter()
            .filter(|edit| source.slice(edit.range()).trim().is_empty())
            .map(|edit| edit.range().len().to_u32())
            .collect();
        assert_eq!(indents.first().copied(), re_seated);
        assert_eq!(indents.len(), usize::from(re_seated.is_some()));
    }

    #[rstest]
    #[case::no_enclosing_callable("class C:\n    marker = super(C, marker)\n")]
    #[case::class_nested_in_a_callable(
        "def f(self):\n    class C:\n        marker = super(C, self)\n"
    )]
    fn degenerate_scope_holds_the_call(#[case] src: &str) {
        assert!(ShedSuperArgs.apply(&parse(src)).is_empty());
    }

    #[test]
    fn deletes_only_the_span_between_the_parentheses() {
        let source = parse("class C:\n    def m(self):\n        return super(C, self).m()\n");
        let groups = ShedSuperArgs.apply(&source);
        let [edit] = groups[0].as_slice() else {
            panic!("a call on one row deletes its span alone");
        };
        assert!(edit.is_deletion());
        assert_eq!(&source.text()[edit.range()], "C, self");
    }

    #[rstest]
    fn reserved_name_binding_holds_every_call(#[values("__class__", "super")] name: &str) {
        let source = parse(&format!(
            "class C:\n    def m(self):\n        {name} = 1\n        return super(C, self).m()\n"
        ));
        assert!(ShedSuperArgs.apply(&source).is_empty());
    }
}
