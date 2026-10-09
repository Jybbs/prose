//! The explicit re-export surface of a module, read from every
//! statement that writes or binds its module-scope `__all__`, from the
//! PEP 484 `x as x` alias form, from an import whose member or source
//! module reads as private, and from a file-level pragma holding every
//! unused import in the module.

use ruff_python_ast::{Alias, Stmt, helpers::is_dunder};
use ruff_text_size::TextRange;

use super::inventory::{ImportNode, is_self_alias};
use crate::{
    primitives::{
        binding::{BindingAnalysis, BindingKind, module_bound_names},
        dunder_all::DunderAll,
    },
    source::Source,
};

/// The tools whose file-level `noqa` head holds the codes that follow
/// it, each beside the casing it reads its own name in. Both read the
/// `noqa` word in either casing.
const NOQA_TOOLS: [(&str, Casing); 2] = [("flake8", Casing::Any), ("ruff", Casing::Lower)];

/// The tool whose file-level pragma names each rule it sets.
const PYRIGHT: &str = "pyright";

/// The values setting a `pyright` rule to report nothing, against the
/// severities that leave it reporting. Both are read in either casing.
const PYRIGHT_OFF: [&str; 2] = ["false", "none"];

/// The `pyright` rule holding every unused import in the file it opens.
const PYRIGHT_UNUSED_IMPORT: &str = "reportUnusedImport";

/// The code `flake8` and its successors report an unread import under,
/// which a `noqa` naming it marks as deliberate.
pub(super) const REEXPORT_CODE: &str = "F401";

/// The names a module marks for re-export.
pub(super) struct Reexports<'a> {
    suppressed: bool,
    surface: DunderAll<'a>,
}

impl<'a> Reexports<'a> {
    /// Reads the module's `__all__` surface and its file-level
    /// pragmas.
    pub(super) fn of(source: &'a Source) -> Self {
        Self {
            suppressed: suppresses_unused_imports(source),
            surface: DunderAll::of(&source.ast().body),
        }
    }

    /// True when the module writes or binds `__all__` anywhere,
    /// whatever the write lists.
    pub(super) fn declares_a_surface(&self) -> bool {
        self.surface.declares_a_surface()
    }

    /// True when `alias`, binding `bound`, marks an explicit re-export,
    /// which a file-level pragma marks for every name at once.
    pub(super) fn holds(&self, alias: &Alias, bound: &str) -> bool {
        self.suppressed || is_self_alias(alias) || self.surface.exports(bound)
    }
}

/// Whether a tool reads the name in its own head in either casing or
/// only in lower case.
#[derive(Clone, Copy)]
enum Casing {
    Any,
    Lower,
}

/// True where `body` binds no name of its own at module scope, reading
/// every shape `module_bound_names` names. A `try` or an `if` is
/// guarding an import rather than defining the module, so neither counts
/// whatever its body binds, and a dunder, a name only an import binds,
/// and a bare annotation binding nothing at run time all stay out.
pub(super) fn defines_no_own_name(analysis: &BindingAnalysis, body: &[Stmt]) -> bool {
    !body.iter().any(|stmt| binds_its_own_name(analysis, stmt))
}

/// True where `alias` takes a name another module may read through
/// this one, meaning a `from`-import of a member whose own name is
/// private or of any member out of a module whose last segment is.
pub(super) fn reexports_a_private_name(node: &ImportNode<'_>, alias: &Alias) -> bool {
    matches!(node, ImportNode::From(_))
        && (is_private(alias.name.as_str())
            || node
                .module()
                .and_then(|module| module.rsplit('.').next())
                .is_some_and(is_private))
}

/// True where `stmt` binds a name the module owns rather than one it
/// carries for a sibling.
fn binds_its_own_name(analysis: &BindingAnalysis, stmt: &Stmt) -> bool {
    if matches!(
        stmt,
        Stmt::If(_) | Stmt::Import(_) | Stmt::ImportFrom(_) | Stmt::Try(_)
    ) {
        return false;
    }
    if stmt
        .as_ann_assign_stmt()
        .is_some_and(|annotated| annotated.value.is_none())
    {
        return false;
    }
    module_bound_names(stmt)
        .into_iter()
        .any(|name| !is_dunder(name) && !only_imported(analysis, name))
}

/// True where `comment` holds every unused import in its file, meaning
/// `pyright`'s spelling or a `ruff` or `flake8` head naming `F401`. A
/// head naming no code suppresses every rule its tool carries, which
/// states nothing about a re-export in particular.
fn holds_unused_imports(comment: &str) -> bool {
    let body = comment.trim_start_matches('#').trim_start();
    if let Some(rules) = past(body, PYRIGHT, Casing::Lower) {
        return rules.split(',').any(turns_unused_imports_off);
    }
    NOQA_TOOLS.iter().any(|(tool, casing)| {
        past(body, tool, *casing)
            .and_then(|rest| past(rest, "noqa", Casing::Any))
            .is_some_and(|codes| {
                codes
                    .split([',', ' ', '\t'])
                    .any(|code| code.eq_ignore_ascii_case(REEXPORT_CODE))
            })
    })
}

/// True where `name` leads with `_` and is no dunder such as
/// `__future__`.
fn is_private(name: &str) -> bool {
    name.starts_with('_') && !is_dunder(name)
}

/// True where every module-scope binding of `name` the analysis records
/// is an import, which leaves a name a guard block imports out of the
/// module's own. A name the analysis records no binding for counts as
/// the module's own, since the statement binding it named it.
fn only_imported(analysis: &BindingAnalysis, name: &str) -> bool {
    let kinds = analysis.module_binding_kinds(name);
    !kinds.is_empty() && kinds.iter().all(|kind| matches!(kind, BindingKind::Import))
}

/// True where `range` opens its own row at column zero, which is where
/// a file-level pragma sits. An indented comment is inside a block and
/// carries no reading of the module as a whole.
fn own_line(source: &Source, range: TextRange) -> bool {
    let text = &source.text()[..usize::from(range.start())];
    text.rsplit_once('\n')
        .map_or(text, |(_, row)| row)
        .is_empty()
}

/// The text of `body` past a leading `word` and the `:` following it,
/// `None` where `body` opens on anything else. The spacing around the
/// `:` is free, which every tool reading one of these heads allows.
fn past<'a>(body: &'a str, word: &str, casing: Casing) -> Option<&'a str> {
    let (head, rest) = body.split_at_checked(word.len())?;
    let matched = match casing {
        Casing::Any => head.eq_ignore_ascii_case(word),
        Casing::Lower => head == word,
    };
    if !matched {
        return None;
    }
    Some(rest.trim_start().strip_prefix(':')?.trim_start())
}

/// True where an own-line comment carries one of the file-level
/// pragmas, which each hold every unused import in the module rather
/// than one statement's.
fn suppresses_unused_imports(source: &Source) -> bool {
    source
        .comment_ranges()
        .iter()
        .any(|range| own_line(source, *range) && holds_unused_imports(source.slice(*range)))
}

/// True where `rule` sets pyright's unused-import rule to report
/// nothing, reading the spacing pyright allows around the `=`.
fn turns_unused_imports_off(rule: &str) -> bool {
    rule.split_once('=').is_some_and(|(name, value)| {
        name.trim() == PYRIGHT_UNUSED_IMPORT
            && PYRIGHT_OFF
                .iter()
                .any(|off| value.trim().eq_ignore_ascii_case(off))
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::listed("__all__ = [\"dumps\"]\n", true)]
    #[case::unlisted("__all__ = [\"other\"]\n", false)]
    #[case::imported_surface("from pkg import __all__\n", true)]
    fn holds_reads_the_dunder_all_surface(#[case] src: &str, #[case] holds: bool) {
        let source = parse(&format!("from json import dumps\n{src}"));
        let alias = &source.ast().body[0]
            .as_import_from_stmt()
            .expect("a from import")
            .names[0];

        assert_eq!(Reexports::of(&source).holds(alias, "dumps"), holds);
    }

    #[rstest]
    #[case::private_module("from _ssl import OPENSSL_VERSION\n", true)]
    #[case::private_submodule("from pkg._impl import thing\n", true)]
    #[case::relative_private("from ._impl import thing\n", true)]
    #[case::parent_relative_private("from .._impl import thing\n", true)]
    #[case::private_member("from subprocess import _args_from_interpreter_flags\n", true)]
    #[case::private_member_aliased("from subprocess import _args as args\n", true)]
    #[case::private_member_of_dots_only("from . import _shared\n", true)]
    #[case::public_member_aliased_private("from quopri import decodestring as _qdecode\n", false)]
    #[case::dunder_member("from pkg import __version__\n", false)]
    #[case::public_module("from pkg.impl import thing\n", false)]
    #[case::public_leaf_of_private("from _pkg.sub import thing\n", false)]
    #[case::dunder_module("from __future__ import annotations\n", false)]
    #[case::dots_only("from . import thing\n", false)]
    #[case::bare_import("import _socket\n", false)]
    fn reexports_a_private_name_reads_the_member_and_the_module_it_comes_from(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let node = ImportNode::of(&source.ast().body[0]).expect("an import statement");
        assert_eq!(reexports_a_private_name(&node, &node.names()[0]), expected);
    }
}
