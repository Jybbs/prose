//! Sheds a class header's redundant base list, dropping an explicit
//! `object` base and the empty parentheses a base-less header carries.
//! A header left with nothing sheds its parentheses alongside the base,
//! whereas a run of `object` bases beside a surviving base or metaclass
//! keyword goes as one span carrying the separator that bound it. An
//! `object` rebound at module scope ahead of the class stays, as does
//! any span carrying a comment.

use ruff_diagnostics::Edit;
use ruff_python_ast::{Expr, StmtClassDef};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        edit::singleton_groups,
        range::{blocks_span, member_deletion_span},
        walk::filter_map_over_stmts,
    },
    rule::{Rule, RuleId},
    source::Source,
};

const OBJECT: &str = "object";

pub(crate) struct ShedRedundantBase;

impl ShedRedundantBase {
    pub(crate) const MESSAGE: &'static str =
        "shed a redundant `object` base or empty class parentheses";

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for ShedRedundantBase {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let per_class = filter_map_over_stmts(&source.ast().body, |stmt| {
            Some(shed(source, stmt.as_class_def_stmt()?))
        });
        singleton_groups(per_class.concat())
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// True when `base` is a bare `object` name.
fn names_object(base: &Expr) -> bool {
    base.as_name_expr().is_some_and(|name| name.id == OBJECT)
}

/// The deletions `class`'s header earns, the whole argument list and the
/// space ahead of it where nothing in it survives, and one span per
/// contiguous run of redundant bases otherwise. A span carrying a
/// comment is dropped, leaving that header as written.
fn shed(source: &Source, class: &StmtClassDef) -> Vec<Edit> {
    let Some(arguments) = class.arguments.as_deref() else {
        return Vec::new();
    };
    let rebound = source
        .binding_analysis()
        .is_defined_before(OBJECT, class.start());
    let redundant = |base: &Expr| !rebound && names_object(base);
    let spans: Vec<TextRange> =
        if arguments.args.iter().filter(|base| redundant(base)).count() == arguments.len() {
            let opener = source.prev_token_end(arguments.start());
            vec![TextRange::new(opener, arguments.end())]
        } else {
            arguments
                .args
                .chunk_by(|a, b| redundant(a) == redundant(b))
                .filter(|run| redundant(&run[0]))
                .map(|run| member_deletion_span(arguments.iter_source_order(), blocks_span(run)))
                .collect()
        };
    spans
        .into_iter()
        .filter(|&span| !source.intersects_comment(span))
        .map(Edit::range_deletion)
        .collect()
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
