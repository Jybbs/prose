//! Sheds a class header's redundant base list, dropping an explicit
//! `object` base and the empty parentheses a base-less header carries.
//! A header left with nothing sheds its parentheses alongside the base,
//! whereas a run of `object` bases beside a surviving base or metaclass
//! keyword goes as one span carrying the separator that bound it. An
//! `object` rebound at module scope ahead of the class stays, as does
//! any span carrying a comment.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, Stmt, StmtClassDef,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{edit::singleton_groups, range::member_deletion_span},
    rule::{Rule, RuleId},
    source::Source,
};

const OBJECT: &str = "object";

pub(crate) struct ShedRedundantBase;

impl ShedRedundantBase {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for ShedRedundantBase {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut walker = Walker {
            edits: Vec::new(),
            source,
        };
        walker.visit_body(&source.ast().body);
        singleton_groups(walker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Walker<'a> {
    edits: Vec<Edit>,
    source: &'a Source,
}

impl Walker<'_> {
    /// Records a deletion of `span`, holding off where `span` carries a
    /// comment.
    fn push_deletion(&mut self, span: TextRange) {
        if !self.source.intersects_comment(span) {
            self.edits.push(Edit::range_deletion(span));
        }
    }

    /// Emits the deletions `class`'s header earns, the whole argument
    /// list and the space ahead of it where nothing in it survives, and
    /// one span per contiguous run of redundant bases otherwise.
    fn shed(&mut self, class: &StmtClassDef) {
        let Some(arguments) = class.arguments.as_deref() else {
            return;
        };
        let rebound = self
            .source
            .binding_analysis()
            .is_defined_before(OBJECT, class.start());
        let redundant = |base: &Expr| !rebound && names_object(base);
        if arguments.args.iter().filter(|base| redundant(base)).count() == arguments.len() {
            let opener = self.source.prev_token_end(arguments.start());
            self.push_deletion(TextRange::new(opener, arguments.end()));
            return;
        }
        let runs: Vec<TextRange> = arguments
            .args
            .chunk_by(|a, b| redundant(a) == redundant(b))
            .filter(|run| redundant(&run[0]))
            .map(|run| TextRange::new(run[0].start(), run.last().expect("non-empty run").end()))
            .collect();
        for run in runs {
            self.push_deletion(member_deletion_span(arguments.iter_source_order(), run));
        }
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::ClassDef(class) = stmt {
            self.shed(class);
        }
        walk_stmt(self, stmt);
    }
}

/// True when `base` is a bare `object` name.
fn names_object(base: &Expr) -> bool {
    base.as_name_expr().is_some_and(|name| name.id == OBJECT)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// The source text each emitted edit deletes, in emission order.
    fn deleted(src: &str) -> Vec<String> {
        let source = parse(src);
        ShedRedundantBase
            .apply(&source)
            .into_iter()
            .flatten()
            .map(|edit| source.text()[edit.range()].to_owned())
            .collect()
    }

    #[rstest]
    #[case("class C(object):\n    pass\n", "(object)")]
    #[case("class C(object,):\n    pass\n", "(object,)")]
    #[case("class C():\n    pass\n", "()")]
    #[case("class C(  ):\n    pass\n", "(  )")]
    #[case("class C (object):\n    pass\n", " (object)")]
    #[case("class C ():\n    pass\n", " ()")]
    fn deletes_the_argument_list_when_no_member_survives(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(deleted(src), [expected]);
    }

    #[rstest]
    #[case("class C(object, metaclass=M):\n    pass\n", "object, ")]
    #[case(
        "class C(\n    object,\n    metaclass=M,\n):\n    pass\n",
        "object,\n    "
    )]
    fn deletes_the_object_and_its_following_separator(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(deleted(src), [expected]);
    }

    #[rstest]
    #[case("class C(Mapping, object):\n    pass\n", ", object")]
    #[case("class C(Mapping, object, object):\n    pass\n", ", object, object")]
    #[case("class C(\n    Mapping,\n    object,\n):\n    pass\n", ",\n    object")]
    fn deletes_the_object_and_its_preceding_separator(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(deleted(src), [expected]);
    }

    #[rstest]
    fn emits_no_edit_where_the_header_holds(
        #[values(
            "class C:\n    pass\n",
            "class C(Mapping):\n    pass\n",
            "class C(*BASES):\n    pass\n",
            "class C(**NAMESPACE):\n    pass\n",
            "class C(metaclass=M):\n    pass\n",
            "class C(  # note\n):\n    pass\n",
            "class C(\n    object,  # note\n):\n    pass\n",
            "object = Legacy\n\n\nclass C(object):\n    pass\n"
        )]
        src: &str,
    ) {
        assert!(deleted(src).is_empty());
    }
}
