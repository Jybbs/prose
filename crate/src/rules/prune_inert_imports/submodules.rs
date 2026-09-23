//! The submodules a module reads through a name bound to their package,
//! each loaded as a side effect of a dotted bare `import` whose own bound
//! name the module may never read.

use std::cell::OnceCell;

use ruff_python_ast::{Alias, ModModule, Stmt, name::UnqualifiedName};
use rustc_hash::{FxHashMap, FxHashSet};

use super::inventory::ImportNode;
use crate::primitives::walk::{Descent, filter_map_over_exprs};

/// True when `alias`, a dotted bare import such as `import a.b.c`, loads
/// the `a.b` submodule an attribute chain in `module` reads, per
/// [`submodule_reads`]. `reads` holds that set once the first dotted
/// import reaches it.
pub(super) fn loads_a_read_submodule(
    node: &ImportNode<'_>,
    alias: &Alias,
    reads: &OnceCell<FxHashSet<String>>,
    module: &ModModule,
) -> bool {
    if !matches!(node, ImportNode::Bare(_)) || alias.asname.is_some() {
        return false;
    }
    let mut segments = alias.name.split('.');
    let (Some(package), Some(submodule)) = (segments.next(), segments.next()) else {
        return false;
    };
    reads
        .get_or_init(|| submodule_reads(module))
        .contains(&format!("{package}.{submodule}"))
}

/// Returns the first two segments, joined with `.`, of the path every
/// attribute chain in `module` reads through a name a module-scope bare
/// `import` binds, so `mp.connection.wait` reads
/// `multiprocessing.connection` under `import multiprocessing as mp`.
fn submodule_reads(module: &ModModule) -> FxHashSet<String> {
    let packages: FxHashMap<&str, &str> = module
        .body
        .iter()
        .filter_map(Stmt::as_import_stmt)
        .flat_map(|node| &node.names)
        .map(|alias| {
            let path = alias.name.as_str();
            match &alias.asname {
                Some(asname) => (asname.as_str(), path),
                None => {
                    let root = path.split('.').next().unwrap_or(path);
                    (root, root)
                }
            }
        })
        .collect();
    if packages.is_empty() {
        return FxHashSet::default();
    }
    filter_map_over_exprs(&module.body, Descent::Into, |expr| {
        if !expr.is_attribute_expr() {
            return None;
        }
        let chain = UnqualifiedName::from_expr(expr)?;
        let (head, rest) = chain.segments().split_first()?;
        let mut path = packages.get(head)?.split('.').chain(rest.iter().copied());
        let (package, submodule) = (path.next()?, path.next()?);
        Some(format!("{package}.{submodule}"))
    })
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::through_an_alias(
        "import multiprocessing as mp\n\n\ndef f():\n    return mp.connection.wait\n",
        &["multiprocessing.connection"],
    )]
    #[case::through_a_dotted_alias("import a.b as x\n\nx.c\n", &["a.b"])]
    #[case::through_the_bound_root("import os.path\n\nos.sep\n", &["os.sep"])]
    #[case::an_unbound_head("value.connection\n", &[])]
    #[case::a_bare_name_alone("import multiprocessing as mp\n\nmp\n", &[])]
    fn submodule_reads_resolves_each_chain_through_its_import(
        #[case] src: &str,
        #[case] expected: &[&str],
    ) {
        let source = parse(src);
        let mut reads: Vec<String> = submodule_reads(source.ast()).into_iter().collect();
        reads.sort();
        assert_eq!(reads, expected);
    }

    #[rstest]
    #[case::read_through_an_alias(
        "import multiprocessing as mp\nimport multiprocessing.connection\n\nmp.connection.wait\n",
        true
    )]
    #[case::unread_submodule(
        "import multiprocessing as mp\nimport multiprocessing.connection\n\nmp.cpu_count\n",
        false
    )]
    #[case::not_dotted(
        "import multiprocessing as mp\nimport connection\n\nmp.connection\n",
        false
    )]
    fn loads_a_read_submodule_keeps_the_import_an_alias_reads_through(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let stmt = &source.ast().body[1];
        let node = ImportNode::of(stmt).expect("an import");
        let alias = &node.names()[0];
        assert_eq!(
            loads_a_read_submodule(&node, alias, &OnceCell::new(), source.ast()),
            expected
        );
    }
}
