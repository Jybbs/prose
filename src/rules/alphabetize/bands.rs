//! Module-scope constant banding. Hoists single-name assignments into
//! a leading band below the imports and a trailing band beneath the
//! definitions, declining whenever the assembled order would seat an
//! eager reference ahead of its definition.

use std::collections::{HashMap, HashSet};

use ruff_python_ast::{Expr, PythonVersion, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::{Ranged, TextRange};

use super::{
    Alphabetize, has_keep_marker,
    tiering::{eval_refs, eval_time_refs, tier_levels},
};
use crate::{
    primitives::{
        binding::{bare_import_bound_name, from_import_bound_name},
        imports::{import_sort_key, same_import_group},
    },
    source::Source,
};

/// The module-scope hoist plan: a band rank per banded statement, the
/// intra-band `(tier, name)` key for each leading and trailing
/// constant, and the eager-reference edges the assembled order keeps
/// backward. Ranks are `0` import, `1` leading constant, `2`
/// definition, `3` trailing constant. A statement absent from `ranks`
/// is an anchor, pinned in place and bounding the bands to its side.
struct BandPlan<'src> {
    edges: Vec<(usize, usize)>,
    keys: HashMap<usize, (usize, &'src str)>,
    ranks: HashMap<usize, u8>,
}

impl BandPlan<'_> {
    /// Appends `region`'s body indices to `out`, sorting the import run
    /// into canonical group order at the front, lifting the leading
    /// constants directly below it, keeping the definitions in their
    /// incoming order, and pooling the trailing constants last. The
    /// import run sorts by `import_sort_key`. Both constant bands sort
    /// by `(tier, name)`. Clears `region`.
    fn drain_region(
        &self,
        body: &[Stmt],
        first_party: &[String],
        region: &mut Vec<usize>,
        out: &mut Vec<usize>,
    ) {
        let mut leading = Vec::new();
        let mut skeleton = Vec::new();
        let mut trailing = Vec::new();
        for idx in region.drain(..) {
            match self.ranks[&idx] {
                1 => leading.push(idx),
                3 => trailing.push(idx),
                _ => skeleton.push(idx),
            }
        }
        leading.sort_by_key(|idx| self.keys[idx]);
        trailing.sort_by_key(|idx| self.keys[idx]);
        let below_imports = skeleton.partition_point(|idx| self.ranks[idx] == 0);
        let (imports, definitions) = skeleton.split_at_mut(below_imports);
        imports.sort_by_key(|&idx| {
            import_sort_key(&body[idx], first_party).expect("rank 0 is import")
        });
        out.extend(imports.iter().copied());
        out.append(&mut leading);
        out.extend(definitions.iter().copied());
        out.append(&mut trailing);
    }

    /// True when every eager reference seats its referent ahead of the
    /// referrer in `order`, the import-safety invariant the hoist holds.
    fn is_sound(&self, order: &[usize]) -> bool {
        let mut position = vec![0usize; order.len()];
        for (slot, &idx) in order.iter().enumerate() {
            position[idx] = slot;
        }
        self.edges
            .iter()
            .all(|&(from, to)| position[to] < position[from])
    }
}

/// A module-scope single-name assignment considered for hoisting,
/// carrying its body index, target name, and the load-context names in
/// its value and its non-deferred annotation. Value references pin the
/// constant when unresolved, whereas annotation references only
/// constrain band order.
struct ConstSite<'src> {
    annot_refs: Vec<&'src str>,
    idx: usize,
    name: &'src str,
    value_refs: Vec<&'src str>,
}

/// Hoists module-level constants into a leading band below the imports
/// and a trailing band beneath the definitions, rewriting `order` in
/// place. A constant rides the leading band when its eval-time surface
/// reaches only imports and fellow leading constants, and the trailing
/// band when it reaches a definition. A statement that is neither an
/// import, a definition, nor a dependency-clean constant pins in place
/// and bounds the bands to its side. Leaves `order` untouched when the
/// plan declines or its assembled order would seat a reference ahead of
/// its definition.
pub(super) fn band_module_constants<'src>(
    source: &'src Source,
    body: &'src [Stmt],
    blocks: &[TextRange],
    first_party: &[String],
    defer_annotations: bool,
    target_version: Option<PythonVersion>,
    order: &mut Vec<usize>,
) -> Option<HashMap<usize, u8>> {
    let plan = module_band_plan(source, body, blocks, defer_annotations, target_version)?;
    let mut banded = Vec::with_capacity(order.len());
    let mut region = Vec::new();
    for &idx in order.iter() {
        if plan.ranks.contains_key(&idx) {
            region.push(idx);
        } else {
            plan.drain_region(body, first_party, &mut region, &mut banded);
            banded.push(idx);
        }
    }
    plan.drain_region(body, first_party, &mut region, &mut banded);
    (plan.is_sound(&banded) && banded != *order).then(|| {
        *order = banded;
        plan.ranks
    })
}

/// The gap the banded order seats after the block of rank `a`, ahead of
/// the block of rank `b`. A constant band stays tight, a definition
/// fronts on two blank lines, and an import run keeps one blank line
/// between canonical groups. `None` falls back to the source gap, the
/// case for a pinned anchor on either side, leaving its spacing intact.
pub(super) fn banded_gap(
    ranks: &HashMap<usize, u8>,
    body: &[Stmt],
    first_party: &[String],
    a: usize,
    b: usize,
) -> Option<&'static str> {
    Some(match (*ranks.get(&a)?, *ranks.get(&b)?) {
        (1, 1) | (3, 3) => "\n",
        (0, 0) if same_import_group(&body[a], &body[b], first_party) => "\n",
        (_, 2) | (2, _) => "\n\n\n",
        _ => "\n\n",
    })
}

/// Returns the target name and optional value of an `Assign` or
/// `AnnAssign` whose target is a single `Name`. `None` for any other
/// shape.
fn assign_run_target(stmt: &Stmt) -> Option<(&str, Option<&Expr>)> {
    match stmt {
        Stmt::AnnAssign(a) => Some((a.target.as_name_expr()?.id.as_str(), a.value.as_deref())),
        Stmt::Assign(a) => {
            let [Expr::Name(name)] = a.targets.as_slice() else {
                return None;
            };
            Some((name.id.as_str(), Some(a.value.as_ref())))
        }
        _ => None,
    }
}

/// Builds the module-scope hoist plan from `body`. Classifies each
/// top-level statement into its band, resolving a single-name
/// assignment's eval-time references against the module's imports,
/// definitions, and fellow constants. A constant whose surface reaches
/// a definition pools into the trailing band, one reaching only clean
/// terminals rides the leading band, and one touching an unresolved
/// name, a reassigned binding, or another anchored constant pins in
/// place. Returns `None` when a constant band's intra-band reference
/// graph carries a cycle, declining the hoist.
fn module_band_plan<'src>(
    source: &'src Source,
    body: &'src [Stmt],
    blocks: &[TextRange],
    defer_annotations: bool,
    target_version: Option<PythonVersion>,
) -> Option<BandPlan<'src>> {
    let analysis = source.binding_analysis();
    let builtins_minor = target_version.unwrap_or_default().minor;
    let suppression = source.suppression_map();
    let mut def_at: HashMap<&'src str, usize> = HashMap::new();
    let mut dup_defs: HashSet<&'src str> = HashSet::new();
    let mut imports: HashSet<&'src str> = HashSet::new();
    let mut ranks: HashMap<usize, u8> = HashMap::new();
    let mut sites: Vec<ConstSite<'src>> = Vec::new();
    for (idx, stmt) in body.iter().enumerate() {
        // A suppressed statement, a `# fmt: skip`-style line, or one a
        // detached comment trails pins in place and bounds the bands to
        // its side, so a single-edit reorder never spans a `# fmt: off`
        // block the pipeline drops the whole edit for, and a free-floating
        // comment never strands away from the statement it sat above.
        // A `#` in the inter-block gap is a detached own-line comment, since
        // `block_range` folds a statement's trailing and attached comments into
        // its own block. `intersects_comment` would over-count a trailing
        // comment whose end touches the gap start.
        let detached_comment = idx > 0
            && source
                .slice(TextRange::new(blocks[idx - 1].end(), blocks[idx].start()))
                .contains('#');
        if detached_comment
            || suppression.intersects(stmt)
            || suppression
                .is_format_suppressed_at(source.line_index(stmt.start()), Alphabetize::SLUG)
        {
            continue;
        }
        match stmt {
            Stmt::ClassDef(StmtClassDef { name, .. })
            | Stmt::FunctionDef(StmtFunctionDef { name, .. }) => {
                if def_at.insert(name.as_str(), idx).is_some() {
                    dup_defs.insert(name.as_str());
                }
                ranks.insert(idx, 2);
            }
            Stmt::Import(node) => {
                imports.extend(node.names.iter().map(bare_import_bound_name));
                ranks.insert(idx, 0);
            }
            Stmt::ImportFrom(node) => {
                imports.extend(node.names.iter().map(from_import_bound_name));
                ranks.insert(idx, 0);
            }
            _ => {
                if let Some((name, value)) = assign_run_target(stmt) {
                    // A `# prose: keep` dict pins its statement, so the
                    // marker freezes module position as well as entry order.
                    if let Some(Expr::Dict(dict)) = value
                        && has_keep_marker(source, dict)
                    {
                        continue;
                    }
                    sites.push(ConstSite {
                        annot_refs: stmt
                            .as_ann_assign_stmt()
                            .filter(|_| !defer_annotations)
                            .map_or_else(Vec::new, |ann| eval_refs(&ann.annotation)),
                        idx,
                        name,
                        value_refs: value.map_or_else(Vec::new, eval_refs),
                    });
                }
            }
        }
    }
    let site_at: HashMap<&'src str, usize> =
        sites.iter().enumerate().map(|(s, c)| (c.name, s)).collect();
    let n = sites.len();
    let mut anchored = vec![false; n];
    let mut reaches_def = vec![false; n];
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (s, site) in sites.iter().enumerate() {
        if analysis.module_reassigned(site.name) {
            anchored[s] = true;
            continue;
        }
        // A value reference to an unresolved name pins the constant unless
        // the name is an import or a builtin, both clean terminals, whereas
        // an annotation reference only ever constrains order, so `x: int = 1`
        // rides the leading band.
        for (refs, anchor_unresolved) in [(&site.value_refs, true), (&site.annot_refs, false)] {
            for &name in refs {
                if name == site.name {
                    continue;
                }
                if dup_defs.contains(name) {
                    anchored[s] = true;
                } else if def_at.contains_key(name) {
                    reaches_def[s] = true;
                } else if let Some(&dep) = site_at.get(name) {
                    deps[s].push(dep);
                } else if anchor_unresolved
                    && !imports.contains(name)
                    && !is_python_builtin(name, builtins_minor, false)
                {
                    anchored[s] = true;
                }
            }
        }
    }
    propagate(&mut anchored, &deps);
    let mut trailing: Vec<bool> = (0..n).map(|s| reaches_def[s] && !anchored[s]).collect();
    propagate(&mut trailing, &deps);
    let mut keys: HashMap<usize, (usize, &'src str)> = HashMap::new();
    for band in [false, true] {
        let members: Vec<usize> = (0..n)
            .filter(|&s| !anchored[s] && trailing[s] == band)
            .collect();
        let local: HashMap<usize, usize> =
            members.iter().enumerate().map(|(at, &s)| (s, at)).collect();
        let dep_sets: Vec<HashSet<usize>> = members
            .iter()
            .map(|&s| {
                deps[s]
                    .iter()
                    .filter_map(|dep| local.get(dep).copied())
                    .collect()
            })
            .collect();
        for (s, tier) in members.iter().copied().zip(tier_levels(&dep_sets)?) {
            keys.insert(sites[s].idx, (tier, sites[s].name));
            ranks.insert(sites[s].idx, if band { 3 } else { 1 });
        }
    }
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for (s, site) in sites.iter().enumerate() {
        if anchored[s] {
            continue;
        }
        for &name in site.value_refs.iter().chain(&site.annot_refs) {
            if let Some(&def) = def_at.get(name) {
                edges.push((site.idx, def));
            } else if let Some(&dep) = site_at.get(name).filter(|&&dep| !anchored[dep]) {
                edges.push((site.idx, sites[dep].idx));
            }
        }
    }
    for (idx, stmt) in body.iter().enumerate() {
        if ranks.get(&idx) == Some(&2) {
            for name in eval_time_refs(stmt, defer_annotations) {
                if let Some(&dep) = site_at.get(name).filter(|&&dep| !anchored[dep]) {
                    edges.push((idx, sites[dep].idx));
                }
            }
        }
    }
    Some(BandPlan { edges, keys, ranks })
}

/// Closes `state` over `deps` to a fixed point, flipping a slot true
/// once any slot it depends on is true, so an initially-seeded flag
/// reaches every slot transitively downstream of a seed.
fn propagate(state: &mut [bool], deps: &[Vec<usize>]) {
    let mut changed = true;
    while changed {
        changed = false;
        for slot in 0..state.len() {
            if !state[slot] && deps[slot].iter().any(|&dep| state[dep]) {
                state[slot] = true;
                changed = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ruff_text_size::TextRange;

    use super::*;
    use crate::primitives::orderer::block_range;
    use crate::testing::parse;

    #[test]
    fn assign_run_target_unwraps_both_assign_kinds_and_filters_non_names() {
        let s = parse("X = 1\nself.x = 1\ny: int = 2\nz: int\n(a, b) = (1, 2)\n");
        let targets: Vec<Option<&str>> = s
            .ast()
            .body
            .iter()
            .map(|s| assign_run_target(s).map(|(name, _)| name))
            .collect();
        assert_eq!(targets, vec![Some("X"), None, Some("y"), Some("z"), None]);
    }

    #[test]
    fn module_band_plan_anchors_a_constant_naming_a_duplicated_definition() {
        let source = parse("def f():\n    pass\n\n\ndef f():\n    pass\n\n\nALIAS = f\n");
        let body = &source.ast().body;
        let blocks: Vec<TextRange> = (0..body.len())
            .map(|i| block_range(&source, body, i, source.module_range()))
            .collect();
        let plan =
            module_band_plan(&source, body, &blocks, false, None).expect("acyclic module plans");
        assert!(
            !plan.ranks.contains_key(&2),
            "ALIAS names an ambiguous f, so it pins in place"
        );
    }

    #[test]
    fn module_band_plan_bands_a_builtin_valued_constant_as_leading() {
        let source = parse("def build():\n    return 1\n\n\nTABLE = dict(timeout=30)\n");
        let body = &source.ast().body;
        let blocks: Vec<TextRange> = (0..body.len())
            .map(|i| block_range(&source, body, i, source.module_range()))
            .collect();
        let plan =
            module_band_plan(&source, body, &blocks, false, None).expect("acyclic module plans");
        assert_eq!(
            plan.ranks[&1], 1,
            "dict is a builtin, so TABLE rides the leading band"
        );
    }

    #[test]
    fn module_band_plan_bands_leading_and_trailing_constants() {
        let source = parse("LEAD = 1\n\n\ndef make():\n    return 1\n\n\nTRAIL = make\n");
        let body = &source.ast().body;
        let blocks: Vec<TextRange> = (0..body.len())
            .map(|i| block_range(&source, body, i, source.module_range()))
            .collect();
        let plan =
            module_band_plan(&source, body, &blocks, false, None).expect("acyclic module plans");
        assert_eq!(plan.ranks[&0], 1, "LEAD touches only a literal");
        assert_eq!(plan.ranks[&1], 2, "make is a definition");
        assert_eq!(plan.ranks[&2], 3, "TRAIL names make");
    }

    #[test]
    fn module_band_plan_declines_a_constant_cycle() {
        let source = parse("A = B\nB = A\n");
        let body = &source.ast().body;
        let blocks: Vec<TextRange> = (0..body.len())
            .map(|i| block_range(&source, body, i, source.module_range()))
            .collect();
        assert!(module_band_plan(&source, body, &blocks, false, None).is_none());
    }

    #[test]
    fn module_band_plan_ignores_a_constant_self_reference() {
        let source = parse("X = X\n");
        let body = &source.ast().body;
        let blocks: Vec<TextRange> = (0..body.len())
            .map(|i| block_range(&source, body, i, source.module_range()))
            .collect();
        let plan = module_band_plan(&source, body, &blocks, false, None)
            .expect("self-reference does not cycle");
        assert_eq!(
            plan.ranks[&0], 1,
            "a self-reference constrains nothing, so X leads"
        );
    }

    #[test]
    fn propagate_flips_slots_reachable_from_a_seed() {
        let deps = vec![vec![], vec![0], vec![1]];
        let mut state = vec![true, false, false];
        propagate(&mut state, &deps);
        assert_eq!(state, vec![true, true, true]);
    }

    #[test]
    fn propagate_leaves_unreached_slots_untouched() {
        let deps = vec![vec![], vec![], vec![]];
        let mut state = vec![false, true, false];
        propagate(&mut state, &deps);
        assert_eq!(state, vec![false, true, false]);
    }
}
