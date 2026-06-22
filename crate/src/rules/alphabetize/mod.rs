//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning. The covered shapes are classes and functions in a body,
//! class-scope `Stmt::AnnAssign` field declarations and `Stmt::Assign`
//! runs with simple `Name` targets, function and lambda keyword-only
//! parameters, call kwargs, set literal elements, import names and their
//! alias lists within each section, `global` and `nonlocal` name lists,
//! `del` target lists, and the
//! string literals inside `__all__` / `__slots__`.
//!
//! Sorting flows through `primitives::orderer::reorder_text`. A
//! recursive `Cow<'src, str>` rewriter folds inner sorts into the
//! outer scope's replacement text, so each outermost reordering scope
//! emits a single edit covering its descendants.
//!
//! Positional-or-keyword parameters never reorder, free function and
//! method alike, because no single-file rewrite can keep every caller's
//! positional binding intact. Only the keyword-only block past `*` sorts.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Decorator, Stmt, StmtAnnAssign, StmtFunctionDef,
    helpers::{any_over_expr, is_compound_statement, is_dunder, map_callable},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        binding::{annotated_name_target, single_name_target, tail_identifier},
        edit::{
            any_owned, apply_inline_edits, narrowed_replacement, singleton_groups, splice_bodies,
        },
        imports::{defers_annotations, import_blank_lines, import_sort_key, sectioned_import_runs},
        orderer::{
            any_sibling_shares_line, assemble_blocks, blocks_span, is_identity, member_block,
            permute_in_place,
        },
        params::pins_positional_params,
        scope::{BodyScope, compound_sub_bodies, scoped_body},
        sections::Sections,
        tiering::permute_defs,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod class_graph;
mod dict;
mod leaves;

use self::{
    class_graph::permute_class_assigns,
    leaves::{collect_docstring_entry_edits, collect_leaf_edits},
};

pub(crate) struct Alphabetize {
    first_party: Vec<String>,
    group_imports: bool,
    sort_docstring_entries: bool,
}

impl Alphabetize {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            first_party: config.first_party(),
            group_imports: config.rules.group_imports.enabled,
            sort_docstring_entries: config.rules.alphabetize.sort_docstring_entries,
        }
    }
}

impl Rule for Alphabetize {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let (mut leaf_edits, param_docs) = collect_leaf_edits(source);
        if self.sort_docstring_entries {
            leaf_edits.extend(collect_docstring_entry_edits(source, &param_docs));
            leaf_edits.sort_unstable();
        }
        let ctx = RewriteCtx {
            defer_annotations: defers_annotations(body),
            first_party: &self.first_party,
            group_imports: self.group_imports,
            leaf_edits: &leaf_edits,
            source,
        };
        let (body_text, body_span) =
            rewrite_body(ctx, body, source.module_range(), BodyScope::Module);
        let edits = match body_text {
            Cow::Borrowed(_) => leaf_edits,
            Cow::Owned(text) => narrowed_replacement(source, body_span, text)
                .into_iter()
                .collect(),
        };
        singleton_groups(edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Invariant context threaded through the body-rewrite recursion.
#[derive(Clone, Copy)]
struct RewriteCtx<'a> {
    defer_annotations: bool,
    first_party: &'a [String],
    group_imports: bool,
    leaf_edits: &'a [Edit],
    source: &'a Source,
}

/// Returns the `StmtAnnAssign` and its target name when the target
/// is a single `Name`.
fn ann_assign_with_named_field(stmt: &Stmt) -> Option<(&StmtAnnAssign, &str)> {
    let ann = stmt.as_ann_assign_stmt()?;
    Some((ann, annotated_name_target(ann)?))
}

/// True when a class body has at least two `Stmt::AnnAssign` field
/// declarations and at least one method whose decorator carries
/// positional arguments.
fn class_pins_methods(body: &[Stmt]) -> bool {
    body.iter()
        .filter(|s| ann_assign_with_named_field(s).is_some())
        .nth(1)
        .is_some()
        && body
            .iter()
            .filter_map(Stmt::as_function_def_stmt)
            .any(pins_positional_params)
}

fn decorator_simple_name(decorator: &Decorator) -> Option<&str> {
    tail_identifier(map_callable(&decorator.expression))
}

/// True when an annotated assignment carries a default, either
/// directly via `= value` or through any nested `Call` in the
/// annotation that carries a `default` or `default_factory` keyword.
fn has_default(ann: &StmtAnnAssign) -> bool {
    ann.value.is_some()
        || any_over_expr(&ann.annotation, |e| {
            e.as_call_expr().is_some_and(|c| {
                c.arguments
                    .keywords
                    .iter()
                    .any(|kw| matches!(kw.arg.as_deref(), Some("default" | "default_factory")))
            })
        })
}

/// Returns the method-group index. `0` for dunders, `1` for
/// `@property` / `@cached_property` (decided by the first decorator),
/// `2` for single-leading-underscore privates, `3` for public.
fn method_group(f: &StmtFunctionDef) -> u8 {
    let name = f.name.as_str();
    if is_dunder(name) {
        0
    } else if f
        .decorator_list
        .first()
        .and_then(decorator_simple_name)
        .is_some_and(|n| matches!(n, "cached_property" | "property"))
    {
        1
    } else if name.starts_with('_') {
        2
    } else {
        3
    }
}

/// Rewrites a non-empty body, returning the rewritten text alongside
/// the block-extent span it covers. The text is `Cow::Owned` when any
/// sibling reorder fires, any descendant rewrite produces owned
/// content, or any leaf edit lands inside, falling back to
/// `Cow::Borrowed` over `source.slice(span)`. `scope` selects which
/// family sorts apply.
fn rewrite_body<'a>(
    ctx: RewriteCtx<'a>,
    body: &[Stmt],
    outer: TextRange,
    scope: BodyScope,
) -> (Cow<'a, str>, TextRange) {
    let RewriteCtx {
        defer_annotations,
        first_party,
        group_imports,
        source,
        ..
    } = ctx;
    let (blocks, rendered): (Vec<TextRange>, Vec<Cow<'a, str>>) = body
        .iter()
        .enumerate()
        .map(|(i, stmt)| {
            let block = member_block(source, body, i, outer);
            (block, rewrite_stmt(ctx, stmt, block, scope))
        })
        .unzip();
    let body_span = blocks_span(&blocks);
    let n = body.len();
    let mut order: Vec<usize> = (0..n).collect();
    let mut import_run_slots: Vec<usize> = Vec::new();
    if !any_sibling_shares_line(source, body) {
        let sections = Sections::of(source, &blocks);
        let in_class = scope == BodyScope::Class;
        if scope != BodyScope::Function {
            for section in sections.ranges() {
                let members = &body[section.clone()];
                permute_defs(&mut order, body, section.clone(), defer_annotations, |s| {
                    s.as_class_def_stmt().map(|c| {
                        let name = c.name.as_str();
                        (name, name)
                    })
                });
                if in_class {
                    permute_class_assigns(&mut order, body, section.clone(), defer_annotations);
                }
                if !(in_class && class_pins_methods(members)) {
                    permute_defs(&mut order, body, section.clone(), defer_annotations, |s| {
                        s.as_function_def_stmt().map(|f| {
                            let name = f.name.as_str();
                            (name, (method_group(f), name))
                        })
                    });
                }
            }
        }
        for run in sectioned_import_runs(&sections, body) {
            permute_in_place(&mut order, body, run, |s| {
                import_sort_key(s, first_party, group_imports)
            });
        }
        // Same-group import neighbors collapse to one line, except across a
        // section marker, whose dividing gap must survive in place.
        import_run_slots.extend((0..n.saturating_sub(1)).filter(|&slot| {
            import_blank_lines(
                &body[order[slot]],
                &body[order[slot + 1]],
                first_party,
                group_imports,
            ) == Some(0)
                && !sections.is_boundary(slot + 1)
        }));
    }
    if !any_owned(&rendered) && is_identity(&order) && import_run_slots.is_empty() {
        return (Cow::Borrowed(source.slice(body_span)), body_span);
    }
    let assembled = assemble_blocks(source, &blocks, &rendered, &order, |i| {
        import_run_slots.binary_search(&i).is_ok().then_some("\n")
    });
    (Cow::Owned(assembled), body_span)
}

/// Recurses into each sub-body of a compound statement, splicing
/// rewritten bodies back into the parent block while leaving header,
/// keyword, and inter-arm regions to leaf-level edits.
fn rewrite_compound<'a>(
    ctx: RewriteCtx<'a>,
    stmt: &Stmt,
    block: TextRange,
    scope: BodyScope,
) -> Cow<'a, str> {
    let bodies = compound_sub_bodies(stmt)
        .into_iter()
        .filter(|(body, _)| !body.is_empty())
        .map(|(body, outer)| rewrite_body(ctx, body, outer, scope));
    splice_bodies(ctx.source, block, bodies, ctx.leaf_edits)
}

/// Rewrites a single statement. Classes and functions fold their body
/// via `rewrite_body` and splice the result. Compound statements
/// (`if`, `for`, `while`, `with`, `try`, `match`) recurse into each
/// sub-body with the inherited `parent_scope`, so module-level reorders
/// (imports, classes, top-level functions) fire inside `if TYPE_CHECKING`
/// and other body-bearing arms. Other shapes apply leaf edits in place.
fn rewrite_stmt<'a>(
    ctx: RewriteCtx<'a>,
    stmt: &Stmt,
    block: TextRange,
    parent_scope: BodyScope,
) -> Cow<'a, str> {
    let Some((body, scope)) = scoped_body(stmt) else {
        if is_compound_statement(stmt) {
            return rewrite_compound(ctx, stmt, block, parent_scope);
        }
        return apply_inline_edits(ctx.source, block, ctx.leaf_edits);
    };
    if body.is_empty() {
        return apply_inline_edits(ctx.source, block, ctx.leaf_edits);
    }
    let (body_text, body_span) = rewrite_body(ctx, body, stmt.range(), scope);
    splice_bodies(ctx.source, block, [(body_text, body_span)], ctx.leaf_edits)
}

/// Returns the simple name assigned by an `Stmt::Assign` whose
/// target is a single `Name`. `None` for multi-target,
/// destructuring, attribute, or subscript targets.
fn simple_name_assign(stmt: &Stmt) -> Option<&str> {
    single_name_target(stmt.as_assign_stmt()?)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, first_class, first_def, parse};

    #[test]
    fn ann_assign_with_named_field_filters_to_name_targets() {
        let s = parse("x: int = 1\nself.x: int = 1\n");
        let names: Vec<Option<&str>> = s
            .ast()
            .body
            .iter()
            .map(|s| ann_assign_with_named_field(s).map(|(_, name)| name))
            .collect();
        assert_eq!(names, vec![Some("x"), None]);
    }

    #[test]
    fn apply_skips_docstring_entry_reorder_when_config_disables_it() {
        let src = indoc! {"
            def f():
                \"\"\"Summary.

                Args:
                    bar: two
                    alpha: one
                \"\"\"
                pass
        "};
        let mut config = Config::default();
        config.rules.alphabetize.sort_docstring_entries = false;
        let rule = Alphabetize::from_config(&config);
        let source = parse(src);
        let edits = rule.apply(&source).into_iter().flatten().collect();
        let text = applied_text(&source, edits);
        let args_section_end = text.find("\"\"\"\n    pass").expect("closer follows args");
        let args_section = &text[..args_section_end];
        let bar_pos = args_section.find("bar: two").expect("bar still present");
        let alpha_pos = args_section
            .find("alpha: one")
            .expect("alpha still present");
        assert!(
            bar_pos < alpha_pos,
            "docstring entries should keep source order when sort-docstring-entries is off",
        );
    }

    #[rstest]
    #[case("@property\ndef f(): pass\n", Some("property"))]
    #[case("@functools.cached_property\ndef f(): pass\n", Some("cached_property"))]
    #[case("@click.option(\"--name\")\ndef f(): pass\n", Some("option"))]
    #[case(
        "@pytest.mark.parametrize(\"a\", [1])\ndef f(): pass\n",
        Some("parametrize")
    )]
    #[case("@functools.wraps(other)\ndef f(): pass\n", Some("wraps"))]
    fn decorator_simple_name_extracts_rightmost_segment(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let s = parse(src);
        let f = first_def(&s);
        let decorator = f.decorator_list.first().expect("one decorator");
        assert_eq!(decorator_simple_name(decorator), expected);
    }

    #[test]
    fn decorator_simple_name_returns_none_for_complex_expressions() {
        let s = parse("@(some_factory())()\ndef f(): pass\n");
        let f = first_def(&s);
        let decorator = f.decorator_list.first().expect("one decorator");
        assert_eq!(decorator_simple_name(decorator), None);
    }

    #[test]
    fn method_group_orders_dunder_property_private_public() {
        let src = indoc! {"
            class C:
                def __init__(self): pass
                @property
                def name(self): pass
                def _helper(self): pass
                def public(self): pass
        "};
        let s = parse(src);
        let class = first_class(&s);
        let groups: Vec<u8> = class
            .body
            .iter()
            .filter_map(Stmt::as_function_def_stmt)
            .map(method_group)
            .collect();
        assert_eq!(groups, vec![0, 1, 2, 3]);
    }

    #[test]
    fn simple_name_assign_filters_to_single_name_targets() {
        let s = parse("X = 1\nself.x = 1\nx, y = 1, 2\n");
        let names: Vec<Option<&str>> = s.ast().body.iter().map(simple_name_assign).collect();
        assert_eq!(names, vec![Some("X"), None, None]);
    }
}
